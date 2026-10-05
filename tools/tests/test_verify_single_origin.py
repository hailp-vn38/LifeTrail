import http.server
import threading
import unittest

from verify_single_origin import verify


INDEX = b'<div id="app"></div><script type="module" src="/assets/app.js"></script>'


class BuiltAppHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path in ("/", "/devices/f273162b-31a4-42db-a0a0-32f342e72a27/day/2026-10-04"):
            self.send_response(200)
            self.send_header("Content-Type", "text/html")
            self.end_headers()
            self.wfile.write(INDEX)
        elif self.path == "/assets/app.js":
            self.send_response(200)
            self.send_header("Content-Type", "application/javascript")
            self.end_headers()
            self.wfile.write(b"console.log('LifeTrail')")
        else:
            self.send_error(404)

    def log_message(self, format, *args):
        pass


class VerifySingleOriginTest(unittest.TestCase):
    def test_requires_same_spa_entrypoint_and_nonempty_asset(self):
        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), BuiltAppHandler)
        thread = threading.Thread(target=server.serve_forever)
        thread.start()
        try:
            verify(
                f"http://127.0.0.1:{server.server_port}",
                "f273162b-31a4-42db-a0a0-32f342e72a27",
                "2026-10-04",
            )
        finally:
            server.shutdown()
            thread.join()
            server.server_close()


if __name__ == "__main__":
    unittest.main()
