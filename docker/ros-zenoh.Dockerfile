FROM osrf/ros:noetic-desktop-full

RUN echo "deb [trusted=yes] https://download.eclipse.org/zenoh/debian-repo/ /" | \
    sudo tee -a /etc/apt/sources.list > /dev/null && \
    sudo apt update

RUN sudo apt install -y systemd systemd-sysv && \
    sudo apt install --no-install-recommends -y zenoh-bridge-ros1 || true

# Patch the zenoh-bridge-ros1 postinst script to exit immediately,
# preventing systemd commands from being run.
RUN sudo sed -i '1i exit 0' /var/lib/dpkg/info/zenoh-bridge-ros1.postinst && \
    sudo dpkg --configure -a
