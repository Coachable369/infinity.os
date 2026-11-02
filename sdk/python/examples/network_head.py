from infinity_network import connect
host = input("Authorized HTTPS hostname: ").strip()
if not host or any(c not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.-" for c in host):
    raise ValueError("Enter a hostname, not a URL")
with connect(host, 443, tls=True) as stream:
    stream.sendall(("HEAD / HTTP/1.1\r\nHost: " + host + "\r\nConnection: close\r\n\r\n").encode("ascii"))
    print(stream.recv(1024).decode("utf-8", errors="replace"))
