#!/usr/bin/env python3
"""Hearth AI Test OS: dependency-free loopback prototype, not an installer."""
import json, os, secrets, shutil, subprocess, sys, threading, urllib.parse, webbrowser, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import metrics

ROOT=Path(__file__).resolve().parent
TOKEN=secrets.token_urlsafe(32)
TOOLS={
 "Claude":("claude",["-p"]),
 "ChatGPT / Codex":("codex",["exec"]),
 "Gemini":("gemini",["-p"]),
 "Local / Ollama":("ollama",["run","llama3.2"])
}
GAMES={"Steam":"steam","Heroic":"heroic","Lutris":"lutris","MangoHud":"mangohud","Gamescope":"gamescope"}
def detect():
 return {
  "agents":[{"name":name,"found":bool(shutil.which(command)),"path":shutil.which(command),"command":command}
            for name,(command,_) in TOOLS.items()],
  "games":[{"name":name,"found":bool(shutil.which(cmd))} for name,cmd in GAMES.items()],
  "telemetry":metrics.snapshot(),
  "system":{"platform":sys.platform,"hostname":os.uname().nodename if hasattr(os,"uname") else "unknown",
            "cpuThreads":os.cpu_count()}
 }
class Handler(BaseHTTPRequestHandler):
 def _respond(self,code,payload,ctype="application/json"):
  data=payload.encode() if isinstance(payload,str) else json.dumps(payload).encode()
  self.send_response(code)
  self.send_header("Content-Type",ctype+"; charset=utf-8")
  self.send_header("Cache-Control","no-store")
  self.send_header("X-Content-Type-Options","nosniff")
  self.send_header("Content-Security-Policy","default-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self'; connect-src 'self'; img-src 'self' data:")
  self.send_header("Content-Length",str(len(data)))
  self.end_headers();self.wfile.write(data)
 def _authorized(self):
  origin=self.headers.get("Origin")
  if origin:
   expected=f"http://127.0.0.1:{self.server.server_port}"
   if origin!=expected: return False
  return secrets.compare_digest(self.headers.get("X-Hearth-Token",""),TOKEN)
 def do_GET(self):
  url=urllib.parse.urlsplit(self.path)
  if url.path=="/" and urllib.parse.parse_qs(url.query).get("token")==[TOKEN]:
   return self._respond(200,(ROOT/"index.html").read_text(),"text/html")
  if url.path=="/app.js":
   return self._respond(200,(ROOT/"app.js").read_text(),"text/javascript")
  if url.path=="/api/status":
   if not self._authorized(): return self._respond(403,{"error":"Forbidden"})
   return self._respond(200,detect())
  return self._respond(404,{"error":"Not found"})
 def do_POST(self):
  if self.path!="/api/ask" or not self._authorized():return self._respond(403,{"error":"Forbidden"})
  if self.headers.get("Content-Type","").split(";")[0].strip()!="application/json":
   return self._respond(415,{"error":"JSON requests only"})
  try:
   length=int(self.headers.get("Content-Length","0"))
   if length<=0 or length>12000: return self._respond(413,{"error":"Invalid prompt size"})
   item=json.loads(self.rfile.read(length))
   if not isinstance(item,dict):
    return self._respond(400,{"error":"Request must be a JSON object"})
   name=item.get("agent");prompt=item.get("prompt")
   if not isinstance(name,str) or name not in TOOLS or not isinstance(prompt,str) or not prompt.strip() or len(prompt)>6000:
    return self._respond(400,{"error":"Select an AI and enter a prompt (max 6000 characters)"})
   exe,args=TOOLS[name]
   resolved=shutil.which(exe)
   if not resolved:return self._respond(404,{"error":f"{name} CLI is not installed or isn't on PATH"})
   # Shell is NEVER used. Only known executables; user must explicitly click Send.
   proc=subprocess.run([resolved,*args,prompt],capture_output=True,text=True,timeout=90,
      cwd=str(Path.home()),env=os.environ.copy())
   return self._respond(200,{"ok":proc.returncode==0,"output":(proc.stdout or proc.stderr)[-24000:],
                                "exitCode":proc.returncode})
  except subprocess.TimeoutExpired:return self._respond(504,{"error":"AI call timed out after 90 seconds"})
  except (ValueError,TypeError,json.JSONDecodeError):return self._respond(400,{"error":"Invalid request"})
  except Exception:
   return self._respond(500,{"error":"AI invocation failed; check the local server terminal for diagnostics"})
 def log_message(self,*args):pass
if __name__=="__main__":
 server=ThreadingHTTPServer(("127.0.0.1",0),Handler)
 addr=f"http://127.0.0.1:{server.server_port}/?token={TOKEN}"
 print("Hearth AI Test OS — local preview only")
 print("Open:",addr,flush=True)
 if "--no-browser" not in sys.argv:threading.Timer(.4,lambda:webbrowser.open(addr)).start()
 try:server.serve_forever()
 except KeyboardInterrupt:pass
 finally:server.server_close()
