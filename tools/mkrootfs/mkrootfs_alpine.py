import os

tarball_url = "https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/alpine-minirootfs-3.22.0-x86_64.tar.gz"

def build(rootfs_dir):
    os.chdir(rootfs_dir)
    os.system("curl \"" + tarball_url + "\" -o TARBALL.tgz")
    os.system("tar xf TARBALL.tgz")
    os.remove("TARBALL.tgz")
    pass
