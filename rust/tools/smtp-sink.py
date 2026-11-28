import os
import socket
import threading

LOG = os.environ.get("SMTP_SINK_LOG", "/tmp/smtp-sink.log")
HOST = os.environ.get("SMTP_SINK_HOST", "127.0.0.1")
PORT = int(os.environ.get("SMTP_SINK_PORT", "2525"))


def append(payload):
    with open(LOG, "ab") as handle:
        handle.write(payload)


def handle(conn):
    buffer = b""
    in_data = False
    conn.sendall(b"220 sink ESMTP\r\n")

    while True:
        try:
            chunk = conn.recv(65536)
        except OSError:
            break

        if not chunk:
            break

        buffer += chunk

        while b"\r\n" in buffer:
            line, buffer = buffer.split(b"\r\n", 1)
            text = line.decode("utf-8", "replace")
            upper = text.upper()

            if in_data:
                if text == ".":
                    in_data = False
                    conn.sendall(b"250 accepted\r\n")
                else:
                    append(line + b"\n")
                continue

            append(("CMD " + text + "\n").encode())

            if upper.startswith("EHLO"):
                conn.sendall(b"250-sink\r\n250-SIZE 10485760\r\n250-8BITMIME\r\n250 PIPELINING\r\n")
            elif upper.startswith("HELO"):
                conn.sendall(b"250 sink\r\n")
            elif upper.startswith("MAIL"):
                conn.sendall(b"250 ok\r\n")
            elif upper.startswith("RCPT"):
                conn.sendall(b"250 ok\r\n")
            elif upper.startswith("DATA"):
                in_data = True
                append(b"=== MESSAGE ===\n")
                conn.sendall(b"354 go ahead\r\n")
            elif upper.startswith("QUIT"):
                conn.sendall(b"221 bye\r\n")
                conn.close()
                return
            else:
                conn.sendall(b"250 ok\r\n")

    conn.close()


def main():
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind((HOST, PORT))
    server.listen(8)
    print(f"smtp sink listening on {HOST}:{PORT}, logging to {LOG}", flush=True)

    while True:
        conn, _ = server.accept()
        threading.Thread(target=handle, args=(conn,), daemon=True).start()


main()
