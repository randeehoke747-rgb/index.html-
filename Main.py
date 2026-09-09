import os
from fastapi import FastAPI

app = FastAPI()

titan_activated = False


def activate_titan():
    global titan_activated

    print("[TITAN] Starting activation...")

    # Your existing Titan initialization goes here

    titan_activated = True
    print("[TITAN] ACTIVATED")


@app.on_event("startup")
async def startup():
    activate_titan()


@app.get("/health")
async def health():
    return {
        "status": "ok",
        "service": "titan",
        "activated": titan_activated
    }
