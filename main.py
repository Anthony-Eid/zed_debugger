import os

current_pid = os.getpid()
print(f"Current process ID: {current_pid}")


def print_hello():
    print("hello world")

    print("Wrong hello world")


def main():
    x = 8

    x = 7

    x = 8

    print_hello()

    while True:
        x = 8
        let = "hello world"


main()
