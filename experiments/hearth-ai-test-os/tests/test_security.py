"""Network/API safety tests; no AI executable is invoked."""
import http.client
import json
import threading
import unittest
from http.server import ThreadingHTTPServer

import server

class TestApiBoundary(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.httpd=ThreadingHTTPServer(("127.0.0.1",0),server.Handler)
        cls.thread=threading.Thread(target=cls.httpd.serve_forever,daemon=True)
        cls.thread.start()
    @classmethod
    def tearDownClass(cls):
        cls.httpd.shutdown()
        cls.httpd.server_close()
        cls.thread.join(timeout=3)
    def request(self,method,path,body=None,headers=None):
        conn=http.client.HTTPConnection("127.0.0.1",self.httpd.server_port,timeout=3)
        conn.request(method,path,body=body,headers=headers or {})
        response=conn.getresponse()
        status=response.status
        response.read()
        conn.close()
        return status
    def test_unauthenticated_status(self):
        self.assertEqual(self.request("GET","/api/status"),403)
    def test_wrong_token(self):
        self.assertEqual(self.request("GET","/api/status",headers={"X-Hearth-Token":"wrong"}),403)
    def test_wrong_origin(self):
        self.assertEqual(self.request("GET","/api/status",headers={"X-Hearth-Token":server.TOKEN,"Origin":"https://malicious.example"}),403)
    def test_json_required(self):
        self.assertEqual(self.request("POST","/api/ask",body="{}",headers={
            "X-Hearth-Token":server.TOKEN,"Content-Type":"text/plain"}),415)
    def test_reject_unapproved_agent(self):
        self.assertEqual(self.request("POST","/api/ask",body=json.dumps({
            "agent":"unapproved","prompt":"hello"}),headers={
            "X-Hearth-Token":server.TOKEN,"Content-Type":"application/json"}),400)
    def test_reject_large_body(self):
        self.assertEqual(self.request("POST","/api/ask",body=" " * 12001,headers={
            "X-Hearth-Token":server.TOKEN,"Content-Type":"application/json"}),413)
if __name__=="__main__":
    unittest.main()
