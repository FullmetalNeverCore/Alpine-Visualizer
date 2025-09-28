from flask import Flask, render_template

app = Flask(__name__)

# @app.route("/")
# def alpine():
#     return render_template("alpine.html", title="Alpine Visualizer")

@app.route("/")
def alpine_lowend():
    return render_template("alpine_lowend.html", title="Alpine Visualizer Low-End")

if __name__ == "__main__":
    app.run(host='0.0.0.0', debug=True, port=5055
    , ssl_context=('192.168.8.145+2.pem','192.168.8.145+2-key.pem'))


