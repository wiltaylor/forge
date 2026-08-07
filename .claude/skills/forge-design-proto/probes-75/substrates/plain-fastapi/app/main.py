from fastapi import FastAPI

from .routes import health

app = FastAPI(title="docsvc")
app.include_router(health.router, prefix="/api")
