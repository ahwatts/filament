# -*- mode: ruby -*-
# vi: set ft=ruby :

# TODO: This needs to install Ruby, Rake, and the "toml" gem on the
# VM. I did it manually via rbenv.

RUST_VERSION = "1.2.0"

GUEST_ADDITIONS_PATH =
  case RbConfig::CONFIG["host_os"]
  when /darwin|mac os/
    "/Applications/VirtualBox.app/Contents/MacOS/VBoxGuestAdditions.iso"
  else
    raise "Add a path for the guest additions for #{RbConfig::CONFIG["host_os"]}"
  end

def common_centos_config(config)
  config.ssh.forward_agent = true

  config.vm.provider :virtualbox do |vb|
    # Add a DVD drive there, so we can update the Guest Additions if that becomes necessary.
    vb.customize [ "storageattach", :id, "--storagectl", "IDE Controller", "--port", 0, "--device", 1, "--type", "dvddrive", "--medium", GUEST_ADDITIONS_PATH ]
  end

  config.vm.provision :shell, inline: <<-_NACHOS
    set +x
    yum -y update
    yum -y groupinstall "Development tools"
    yum -y install kernel-devel
    if [ -f /etc/pki/tls/certs/ca-bundle.crt.rpmnew ]; then
        mv /etc/pki/tls/certs/ca-bundle.crt.rpmnew /etc/pki/tls/certs/ca-bundle.crt
    fi
  _NACHOS

  config.vm.provision :shell, inline: <<-_NACHOS
    set +x
    echo "/usr/local/lib" > /etc/ld.so.conf.d/local.conf
    ldconfig
  _NACHOS

  config.vm.provision :shell, inline: <<-_NACHOS
    set -x

    rust_version="$(/usr/local/bin/rustc --version | awk '{ print $2; }')"

    if [ "#{RUST_VERSION}" != "$rust_version" ]; then
        if [ -x /usr/local/lib/rustlib/uninstall.sh ]; then
            /usr/local/lib/rustlib/uninstall.sh
        fi

        rm -rf /tmp/rust_install
        mkdir -p /tmp/rust_install
        cd /tmp/rust_install

        dist_dir=rust-#{RUST_VERSION}-$(uname -p)-unknown-linux-gnu
        dist_file=$dist_dir.tar.gz
        wget -nv https://static.rust-lang.org/dist/$dist_file
        tar -xzf $dist_file

        cd $dist_dir
        ./install.sh
    fi
  _NACHOS
end

Vagrant.configure(2) do |config|
  config.vm.define "centos6" do |c6|
    c6.vm.box = "chef/centos-6.6"
    common_centos_config(c6)
  end

  # Due to the GLIBC 2.14 memcpy / memmove scandal, we should probably
  # just build on CentOS 6, so that we don't get the Bad Symbol.

  # config.vm.define "centos7" do |c7|
  #   c7.vm.box = "chef/centos-7.0"
  #   common_centos_config(c7)
  # end
end
