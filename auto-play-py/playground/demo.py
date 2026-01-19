import sys

try:
    import auto_play
except ImportError:
    print(
        "Could not import auto_play. Please ensure it is installed in your current environment."
    )
    sys.exit(1)


def main():
    print(
        "Attempting to connect to ADB device (using default localhost:5555 for demo)..."
    )
    # Replace with your actual device serial if needed
    serial = "192.168.1.3:40919"

    try:
        # 1. Connect
        ap = auto_play.AutoPlay.connect(serial)
        print("Connected successfully!")

        # 2. Get Info
        sdk = ap.get_sdk()
        abi = ap.get_abi()
        print(f"Device SDK: {sdk}")
        print(f"Device ABI: {abi}")

        # 3. Control
        if ap.is_screen_on():
            print("Screen is currently ON")
        else:
            print("Screen is OFF, attempting to wake up...")
            ap.ensure_screen_on()
            assert ap.is_screen_on()

        # 4. Screenshot
        print("Capturing screenshot...")
        png_data = ap.screencap()
        print(f"Got screenshot data: {len(png_data)} bytes")

        # Optional: Save it if PIL is available
        try:
            import io

            from PIL import Image

            img = Image.open(io.BytesIO(png_data))
            img.show()
            print("Screenshot displayed.")
        except ImportError:
            print("Install Pillow to view the screenshot.")

    except RuntimeError as e:
        print(f"Runtime error (ADB might not be connected): {e}")


if __name__ == "__main__":
    main()
