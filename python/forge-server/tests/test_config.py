"""The contract's environment variables: one declaration, contract defaults.

Pins every default to the literal value docs/api-contract.md documents, so
this backend cannot drift from the contract (or from the Rust backend, whose
own tests pin the same values) while this file is green.
"""

import pytest

from forge_server import config

CONTRACT_VARS = [
    "FORGE_JWT_SECRET",
    "FORGE_AUTH_USERS",
    "FORGE_JWT_TTL_SECS",
    "FORGE_JWT_ISS",
    "FORGE_HOST",
    "FORGE_PORT",
    "FORGE_DATA_DIR",
    "FORGE_COMPONENTS_DIR",
    "FORGE_CORS_ORIGINS",
]


@pytest.fixture(autouse=True)
def clean_env(monkeypatch):
    for name in CONTRACT_VARS:
        monkeypatch.delenv(name, raising=False)


def test_defaults_match_the_contract_table():
    assert config.DEFAULT_TTL_SECS == 86400
    assert config.DEFAULT_ISS == "forge"
    assert config.DEFAULT_HOST == "127.0.0.1"
    assert config.DEFAULT_PORT == 8765
    assert config.DEFAULT_DATA_DIR == "./data"
    assert config.DEFAULT_COMPONENTS_DIR == "./components"
    assert config.DEFAULT_CORS_ORIGINS == [
        "http://localhost:5173",
        "http://127.0.0.1:5173",
    ]


def test_unset_variables_yield_the_defaults():
    assert config.jwt_secret() is None
    assert config.auth_users() == ""
    assert config.jwt_ttl_secs() == config.DEFAULT_TTL_SECS
    # No default for the issuer: it is validated only when set explicitly,
    # so the caller must see the difference between unset and "forge".
    assert config.jwt_iss() is None
    assert config.host() == config.DEFAULT_HOST
    assert config.port() == config.DEFAULT_PORT
    assert config.data_dir() == config.DEFAULT_DATA_DIR
    assert config.components_dir() == config.DEFAULT_COMPONENTS_DIR
    assert config.cors_origins() == config.DEFAULT_CORS_ORIGINS


def test_set_variables_win(monkeypatch):
    monkeypatch.setenv("FORGE_JWT_SECRET", "s" * 32)
    monkeypatch.setenv("FORGE_AUTH_USERS", "admin:pw")
    monkeypatch.setenv("FORGE_JWT_TTL_SECS", "60")
    monkeypatch.setenv("FORGE_JWT_ISS", "my-issuer")
    monkeypatch.setenv("FORGE_HOST", "0.0.0.0")
    monkeypatch.setenv("FORGE_PORT", "9000")
    monkeypatch.setenv("FORGE_DATA_DIR", "/srv/data")
    monkeypatch.setenv("FORGE_COMPONENTS_DIR", "/srv/components")
    monkeypatch.setenv("FORGE_CORS_ORIGINS", "https://a.example, https://b.example")
    assert config.jwt_secret() == "s" * 32
    assert config.auth_users() == "admin:pw"
    assert config.jwt_ttl_secs() == 60
    assert config.jwt_iss() == "my-issuer"
    assert config.host() == "0.0.0.0"
    assert config.port() == 9000
    assert config.data_dir() == "/srv/data"
    assert config.components_dir() == "/srv/components"
    assert config.cors_origins() == ["https://a.example", "https://b.example"]


def test_a_non_numeric_port_or_ttl_is_an_error(monkeypatch):
    monkeypatch.setenv("FORGE_PORT", "not-a-port")
    with pytest.raises(ValueError, match="FORGE_PORT"):
        config.port()
    monkeypatch.setenv("FORGE_JWT_TTL_SECS", "soon")
    with pytest.raises(ValueError, match="FORGE_JWT_TTL_SECS"):
        config.jwt_ttl_secs()
