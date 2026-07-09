#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx>=0.27", "pyjwt[crypto]>=2.9"]
# ///
"""End-to-end test of forge-auth over real HTTP.

Covers: admin API, authorization code + PKCE, JWKS-verified RS256 tokens,
userinfo, refresh rotation, RFC 8693 token exchange (incl. legacy HS256), and
upstream OIDC federation — by running a SECOND forge-auth instance as the
upstream identity provider (which exercises discovery, upstream PKCE, nonce
and id_token verification in the openidconnect connector).

Run via `just e2e-test` (builds the debug binary itself).
"""

import base64
import hashlib
import os
import secrets
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from urllib.parse import parse_qs, urlparse

import httpx
import jwt as pyjwt

REPO = Path(__file__).resolve().parent.parent
PORT_A = 18870  # IdP under test
PORT_B = 18871  # upstream IdP (federation test)
ISSUER_A = f"http://127.0.0.1:{PORT_A}"
ISSUER_B = f"http://127.0.0.1:{PORT_B}"
ADMIN_PW = "e2e-admin-password"

passed = 0


def check(name: str, condition: bool, detail: str = ""):
    global passed
    if not condition:
        print(f"FAIL {name} {detail}")
        sys.exit(1)
    passed += 1
    print(f"ok   {name}")


def build_binary() -> Path:
    print("building debug binary…")
    subprocess.run(["cargo", "build", "-p", "forge-auth"], cwd=REPO, check=True)
    return REPO / "target/debug/forge-auth"


def start_server(binary: Path, port: int, issuer: str, workdir: Path, database_url: str | None = None) -> subprocess.Popen:
    env = os.environ.copy()
    env.update(
        {
            "DATABASE_URL": database_url or f"sqlite://{workdir}/forge-auth.db?mode=rwc",
            "FORGE_AUTH_ISSUER": issuer,
            "FORGE_AUTH_ADMIN_PASSWORD": ADMIN_PW,
            "FORGE_AUTH_COOKIE_SECURE": "false",
            "FORGE_HOST": "127.0.0.1",
            "FORGE_PORT": str(port),
        }
    )
    log = open(workdir / "server.log", "w")
    proc = subprocess.Popen([binary], env=env, cwd=workdir, stdout=log, stderr=log)
    for _ in range(100):
        try:
            httpx.get(f"http://127.0.0.1:{port}/api/health", timeout=1)
            return proc
        except httpx.HTTPError:
            time.sleep(0.1)
    print(f"server on :{port} did not come up; log:")
    print((workdir / "server.log").read_text())
    sys.exit(1)


def client_for(port: int) -> httpx.Client:
    c = httpx.Client(base_url=f"http://127.0.0.1:{port}", follow_redirects=False, timeout=10)
    c.headers["X-Forge-Auth"] = "1"
    return c


def admin_login(c: httpx.Client):
    r = c.post("/api/login", json={"username": "admin", "password": ADMIN_PW})
    check("admin login", r.status_code == 200, r.text)


def api(c: httpx.Client, method: str, path: str, **kwargs) -> dict:
    r = c.request(method, path, **kwargs)
    body = r.json()
    if r.status_code != 200 or body.get("ok") is False:
        print(f"FAIL api {method} {path}: {r.status_code} {r.text}")
        sys.exit(1)
    return body["data"]


def pkce() -> tuple[str, str]:
    verifier = secrets.token_urlsafe(48)
    challenge = base64.urlsafe_b64encode(hashlib.sha256(verifier.encode()).digest()).rstrip(b"=").decode()
    return verifier, challenge


def main():
    binary = build_binary()
    tmp = Path(tempfile.mkdtemp(prefix="forge-auth-e2e-"))
    (tmp / "a").mkdir()
    (tmp / "b").mkdir()
    procs = []
    try:
        # Postgres parity: FORGE_AUTH_E2E_DB_A=postgres://... runs the primary
        # instance on Postgres (instance B stays on SQLite).
        procs.append(start_server(binary, PORT_A, ISSUER_A, tmp / "a", os.environ.get("FORGE_AUTH_E2E_DB_A")))
        procs.append(start_server(binary, PORT_B, ISSUER_B, tmp / "b"))
        run_tests()
        print(f"\nall {passed} checks passed")
    finally:
        for p in procs:
            p.terminate()
        shutil.rmtree(tmp, ignore_errors=True)


def run_tests():
    a = client_for(PORT_A)
    admin_login(a)

    # --- discovery + JWKS ---
    disco = httpx.get(f"{ISSUER_A}/.well-known/openid-configuration").json()
    check("discovery issuer", disco["issuer"] == ISSUER_A)
    jwks = httpx.get(disco["jwks_uri"]).json()
    check("jwks has RS256 key", jwks["keys"][0]["alg"] == "RS256")

    # --- admin: role, user, clients ---
    api(a, "POST", "/api/admin/roles", json={"name": "media"})
    user = api(a, "POST", "/api/admin/users", json={
        "username": "alice", "password": "alice-password-1",
        "email": "alice@example.test", "email_verified": True, "roles": ["media"],
    })
    check("user created with role", user["roles"] == ["media"])

    rp = api(a, "POST", "/api/admin/clients", json={
        "id": "rp-app", "name": "RP App", "trusted": True,
        "redirect_uris": ["http://rp.test/cb"],
        "exchange_audiences": ["media-app", "legacy-app"],
    })
    rp_secret = rp["client_secret"]
    api(a, "POST", "/api/admin/clients", json={
        "id": "media-app", "name": "Media", "trusted": True,
        "role_mappings": {"media": "library-user"},
    })
    api(a, "POST", "/api/admin/clients", json={
        "id": "legacy-app", "name": "Legacy Forge App",
        "legacy_hs256_secret": "legacy-shared-secret-32-characters!",
    })

    # --- full code + PKCE flow as alice ---
    alice = client_for(PORT_A)
    verifier, challenge = pkce()
    r = alice.get("/oauth2/authorize", params={
        "response_type": "code", "client_id": "rp-app",
        "redirect_uri": "http://rp.test/cb", "scope": "openid profile email roles",
        "state": "st4te", "nonce": "n0nce",
        "code_challenge": challenge, "code_challenge_method": "S256",
    })
    check("authorize → login redirect", r.status_code == 303 and r.headers["location"].startswith("/login?request="))
    request_id = r.headers["location"].split("=")[1]

    info = alice.get(f"/api/login/request/{request_id}").json()["data"]
    check("login request info", info["client_name"] == "RP App")

    r = alice.post("/api/login", json={"username": "alice", "password": "alice-password-1", "request_id": request_id})
    check("alice login", r.status_code == 200, r.text)
    redirect_to = r.json()["data"]["redirect_to"]
    check("login resumes authorize", redirect_to == f"/oauth2/authorize?request={request_id}")

    r = alice.get(redirect_to)
    check("authorize issues code", r.status_code == 303 and r.headers["location"].startswith("http://rp.test/cb"))
    q = parse_qs(urlparse(r.headers["location"]).query)
    check("state round-trips", q["state"] == ["st4te"])
    code = q["code"][0]

    r = httpx.post(f"{ISSUER_A}/oauth2/token", data={
        "grant_type": "authorization_code", "code": code,
        "redirect_uri": "http://rp.test/cb", "code_verifier": verifier,
        "client_id": "rp-app", "client_secret": rp_secret,
    })
    check("token endpoint", r.status_code == 200, r.text)
    tokens = r.json()
    check("token response shape", all(k in tokens for k in ("access_token", "refresh_token", "id_token")))

    # Verify against the real JWKS with PyJWT.
    key = pyjwt.PyJWKClient(disco["jwks_uri"]).get_signing_key_from_jwt(tokens["access_token"]).key
    claims = pyjwt.decode(tokens["access_token"], key, algorithms=["RS256"], audience="rp-app", issuer=ISSUER_A)
    check("access token verifies via JWKS", claims["preferred_username"] == "alice")
    check("roles in claims", claims["roles"] == ["media"])
    id_claims = pyjwt.decode(tokens["id_token"], key, algorithms=["RS256"], audience="rp-app", issuer=ISSUER_A)
    check("id_token nonce", id_claims["nonce"] == "n0nce")

    r = httpx.get(f"{ISSUER_A}/oauth2/userinfo", headers={"Authorization": f"Bearer {tokens['access_token']}"})
    check("userinfo", r.status_code == 200 and r.json()["email"] == "alice@example.test")

    # --- refresh rotation + reuse detection ---
    r = httpx.post(f"{ISSUER_A}/oauth2/token", data={
        "grant_type": "refresh_token", "refresh_token": tokens["refresh_token"],
        "client_id": "rp-app", "client_secret": rp_secret,
    })
    check("refresh", r.status_code == 200, r.text)
    new_refresh = r.json()["refresh_token"]
    check("refresh rotates", new_refresh != tokens["refresh_token"])
    r = httpx.post(f"{ISSUER_A}/oauth2/token", data={
        "grant_type": "refresh_token", "refresh_token": tokens["refresh_token"],
        "client_id": "rp-app", "client_secret": rp_secret,
    })
    check("refresh reuse rejected", r.status_code == 400 and r.json()["error"] == "invalid_grant")
    r = httpx.post(f"{ISSUER_A}/oauth2/token", data={
        "grant_type": "refresh_token", "refresh_token": new_refresh,
        "client_id": "rp-app", "client_secret": rp_secret,
    })
    check("family revoked after reuse", r.status_code == 400)

    # --- RFC 8693 token exchange (role selection per target client) ---
    r = httpx.post(f"{ISSUER_A}/oauth2/token", data={
        "grant_type": "urn:ietf:params:oauth:grant-type:token-exchange",
        "subject_token": tokens["access_token"],
        "subject_token_type": "urn:ietf:params:oauth:token-type:access_token",
        "audience": "media-app",
        "client_id": "rp-app", "client_secret": rp_secret,
    })
    check("token exchange", r.status_code == 200, r.text)
    ex = pyjwt.decode(r.json()["access_token"], key, algorithms=["RS256"], audience="media-app", issuer=ISSUER_A)
    check("exchange maps roles for target", ex["roles"] == ["library-user"])
    check("exchange sets azp", ex["azp"] == "rp-app")

    # Legacy HS256 exchange for stock forge apps.
    r = httpx.post(f"{ISSUER_A}/oauth2/token", data={
        "grant_type": "urn:ietf:params:oauth:grant-type:token-exchange",
        "subject_token": tokens["access_token"],
        "audience": "legacy-app",
        "client_id": "rp-app", "client_secret": rp_secret,
    })
    check("legacy exchange", r.status_code == 200, r.text)
    legacy = pyjwt.decode(
        r.json()["access_token"], "legacy-shared-secret-32-characters!",
        algorithms=["HS256"], options={"verify_aud": False},
    )
    check("legacy token forge-shaped", legacy["sub"] == "alice" and legacy["roles"] == ["media"])

    # --- introspection + revocation ---
    r = httpx.post(f"{ISSUER_A}/oauth2/introspect", data={
        "token": tokens["access_token"], "client_id": "rp-app", "client_secret": rp_secret,
    })
    check("introspect active", r.json()["active"] is True)
    r = httpx.post(f"{ISSUER_A}/oauth2/revoke", data={
        "token": new_refresh, "client_id": "rp-app", "client_secret": rp_secret,
    })
    check("revoke returns 200", r.status_code == 200)

    # --- upstream OIDC federation: instance B is the upstream IdP ---
    b = client_for(PORT_B)
    admin_login(b)
    api(b, "POST", "/api/admin/clients", json={
        "id": "forge-auth-a", "name": "Forge Auth A", "trusted": True,
        "redirect_uris": [f"{ISSUER_A}/api/callback/upstream-b"],
    })
    # (re-read the secret from the create response)
    # recreate deterministic: fetch secret by regenerating
    up_secret = api(b, "POST", "/api/admin/clients/forge-auth-a/secret")["client_secret"]

    api(a, "POST", "/api/admin/providers", json={
        "slug": "upstream-b", "kind": "oidc", "display_name": "Upstream B",
        "allow_signup": True, "link_by_email": False,
        "config": {
            "issuer_url": ISSUER_B, "client_id": "forge-auth-a",
            "client_secret": up_secret, "scopes": "openid profile email",
        },
    })

    # Browser simulation: fresh cookie jars on both instances.
    browser_a = client_for(PORT_A)
    browser_b = client_for(PORT_B)
    r = browser_a.get("/api/login/upstream/upstream-b")
    check("upstream start redirects to B", r.status_code == 303 and r.headers["location"].startswith(ISSUER_B), r.text)

    # At B: authorize → login page → login as B's admin → resume → callback to A.
    url = r.headers["location"]
    r = browser_b.get(url)
    check("B authorize wants login", r.status_code == 303 and r.headers["location"].startswith("/login?request="))
    b_request = r.headers["location"].split("=")[1]
    r = browser_b.post("/api/login", json={"username": "admin", "password": ADMIN_PW, "request_id": b_request})
    check("login at B", r.status_code == 200, r.text)
    r = browser_b.get(r.json()["data"]["redirect_to"])
    check("B issues code to A's callback", r.status_code == 303 and r.headers["location"].startswith(f"{ISSUER_A}/api/callback/upstream-b"), r.headers.get("location", ""))

    callback = r.headers["location"]
    r = browser_a.get(callback.removeprefix(ISSUER_A))
    check("A accepts federated login", r.status_code == 303 and r.headers["location"] == "/account", f"{r.status_code} {r.headers.get('location')} {r.text}")

    session = browser_a.get("/api/session").json()["data"]
    check("federated session established", session["authenticated"] is True)
    check("federated amr", session["user"]["amr"] == ["federated:upstream-b"])
    check("JIT-provisioned user", session["user"]["username"] == "admin2" or session["user"]["username"].startswith("admin"), session["user"]["username"])

    # Provider test endpoint (runs discovery against B).
    providers = api(a, "GET", "/api/admin/providers")
    pid = providers[0]["id"]
    result = api(a, "POST", f"/api/admin/providers/{pid}/test")
    check("provider test: discovery ok", result["ok"] is True, str(result))


if __name__ == "__main__":
    main()
