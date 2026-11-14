from __future__ import annotations

from flask import Flask, jsonify

from service import register_hackathon_sponsor

app = Flask(__name__)
sponsor = register_hackathon_sponsor(app)


@app.get("/healthz")
def healthz():
    status = sponsor.status()
    return jsonify({"status": "ok", **status}), 200
