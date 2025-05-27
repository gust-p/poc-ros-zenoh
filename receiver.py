import zenoh


def listener(sample):
    print(
        f"Received {sample.kind} ('{sample.key_expr}': '{sample.payload.to_string()}')"
    )


if __name__ == "__main__":
    print("Zenoh subscriber started. Press 'q' + Enter to quit.")

    with zenoh.open(zenoh.Config()) as session:
        sub = session.declare_subscriber("rt/hello", listener)

        try:
            while True:
                user_input = input()
                if user_input.lower() == "q":
                    break
        except KeyboardInterrupt:
            pass

        print("Shutting down subscriber...")
