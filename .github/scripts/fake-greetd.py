#!/usr/bin/python3
"""A stand-in greetd for CI (login-greetd-test.yml): greetd's IPC on a Unix socket (GREETD_SOCK), so Hearth's greeter
(Quickshell's Greetd service) can sign in without PAM or a real session. Each message: a native-endian 32-bit length,
then JSON (greetd-ipc(7)). Made-up accounts: alex (password "hearth"), sam (locked, as pam_faillock says it), anyone
else: wrong password. What happened goes to the log as JSON lines; answers are never written there, only whether they
were right.

  fake-greetd.py SOCKET LOG
"""
import json
import os
import socket
import struct
import sys

PASSWORDS = {'alex': 'hearth', 'jordan': 'hearth'}
LOCKED = {'sam'}


def send(conn, obj):
    data = json.dumps(obj).encode()
    conn.sendall(struct.pack('=I', len(data)) + data)


def recv(conn):
    head = b''
    while len(head) < 4:
        chunk = conn.recv(4 - len(head))
        if not chunk:
            return None
        head += chunk
    (n,) = struct.unpack('=I', head)
    data = b''
    while len(data) < n:
        chunk = conn.recv(n - len(data))
        if not chunk:
            return None
        data += chunk
    return json.loads(data.decode())


def main(sock_path, log_path):
    if os.path.exists(sock_path):
        os.remove(sock_path)
    srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    srv.bind(sock_path)
    srv.listen(4)
    log = open(log_path, 'a', buffering=1)

    def note(**kw):
        log.write(json.dumps(kw) + '\n')
    note(event='listening')
    while True:
        conn, _ = srv.accept()
        note(event='connected')
        user, stage = None, None
        while True:
            req = recv(conn)
            if req is None:
                note(event='disconnected')
                break
            t = req.get('type')
            if t == 'create_session':
                user = req.get('username')
                note(event='create_session', user=user)
                if user in LOCKED:
                    stage = 'locked-info'
                    send(conn, {'type': 'auth_message', 'auth_message_type': 'error',
                                'auth_message': 'The account is locked due to 3 failed logins.'})
                else:
                    stage = 'password'
                    send(conn, {'type': 'auth_message', 'auth_message_type': 'secret', 'auth_message': 'Password: '})
            elif t == 'post_auth_message_response':
                if stage == 'locked-info':
                    stage = 'password'
                    send(conn, {'type': 'auth_message', 'auth_message_type': 'secret', 'auth_message': 'Password: '})
                    continue
                right = user in PASSWORDS and req.get('response') == PASSWORDS[user]
                note(event='answer', user=user, right=right, given=req.get('response') is not None)
                if right:
                    stage = 'ready'
                    send(conn, {'type': 'success'})
                else:
                    stage = None
                    send(conn, {'type': 'error', 'error_type': 'auth_error', 'description': 'pam_authenticate: AUTH_ERR'})
            elif t == 'start_session':
                note(event='start_session', user=user, cmd=req.get('cmd'), env=req.get('env'), ready=stage == 'ready')
                if stage == 'ready':
                    send(conn, {'type': 'success'})
                else:
                    send(conn, {'type': 'error', 'error_type': 'error', 'description': 'no session ready'})
            elif t == 'cancel_session':
                note(event='cancel_session', user=user)
                user, stage = None, None
                send(conn, {'type': 'success'})
            else:
                send(conn, {'type': 'error', 'error_type': 'error', 'description': 'unknown request'})
        conn.close()


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
