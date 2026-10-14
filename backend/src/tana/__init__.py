def main() -> None:
    import uvicorn

    uvicorn.run("tana.app:app", host="0.0.0.0", port=8000, workers=1)
