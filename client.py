import socket
import time

s = socket.socket()
s.settimeout(5.0)  # 5 second timeout

try:
    print("Attempting to connect to 127.0.0.1:9001...")
    s.connect(("127.0.0.1", 9001))
    print("Connected successfully!")
except Exception as e:
    print(f"Failed to connect: {e}")
    exit(1)

# Read the welcome message
try:
    print("Waiting for welcome message...")
    welcome = s.recv(1024)
    print(f"Received {len(welcome)} bytes")
    if welcome:
        print(f"Server says: {welcome.decode().strip()}")
    else:
        print("Server closed connection immediately (received 0 bytes)")
        exit(1)
except socket.timeout:
    print("Timeout waiting for welcome message - server didn't send anything")
    exit(1)
except Exception as e:
    print(f"Error reading welcome: {e}")
    exit(1)

for i in range(3):  # Just 3 messages for testing
    msg = f"message {i}\n"
    try:
        print(f"\nSending: {msg.strip()}")
        s.sendall(msg.encode())
        print("Sent successfully")

        print("Waiting for response...")
        response = s.recv(1024)
        if not response:
            print(f"Server closed connection after message {i}")
            break
        print(f"Server says: {response.decode().strip()}")

    except socket.timeout:
        print(f"Timeout waiting for response to message {i}")
        break
    except BrokenPipeError:
        print(f"Connection closed by server at message {i}")
        break
    except Exception as e:
        print(f"Error at message {i}: {e}")
        break

    time.sleep(0.5)

try:
    s.close()
    print("\nConnection closed cleanly")
except Exception as e:
    print(f"Error closing socket: {e}")