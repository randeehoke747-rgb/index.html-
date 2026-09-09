
from flask import Flask, render_template

app = Flask(__name__)@app.route("/health", methods=["GET"])
def health():
    return {
        "status": "ok",
        "service": "titan",
        "activated": True
    }, 200

@app.route('/')
def home():
    return render_template('index.html')

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5000)
