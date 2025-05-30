FROM ros:kilted

SHELL ["/bin/bash", "-o", "pipefail", "-o", "errexit", "-c"]

RUN apt update && apt upgrade -y 
RUN apt install ros-kilted-turtlesim -y

ENTRYPOINT []
