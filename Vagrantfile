# -*- mode: ruby -*-
# vi: set ft=ruby :

RUST_VERSION = "1.5.0"
BOX_RUBY_VERSION = "2.2.3"

def common_centos_config(config)
  config.ssh.forward_agent = true

  # Install basic dev tools.
  config.vm.provision :shell, inline: <<-_NACHOS
    set +x
    yum -y update
    yum -y groupinstall "Development tools"
    yum -y install kernel-devel
    if [ -f /etc/pki/tls/certs/ca-bundle.crt.rpmnew ]; then
        mv /etc/pki/tls/certs/ca-bundle.crt.rpmnew /etc/pki/tls/certs/ca-bundle.crt
    fi
  _NACHOS

  # Make sure /usr/local/lib is in the default LD_LIBRARY_PATH.
  config.vm.provision :shell, inline: <<-_NACHOS
    set +x
    echo "/usr/local/lib" > /etc/ld.so.conf.d/local.conf
    ldconfig
  _NACHOS

  # Install / upgrade the rust compiler.
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

  # Install / upgrade Ruby and the gems we need for the Rakefile via rbenv.
  config.vm.provision :shell, privileged: false, inline: <<-_NACHOS
    set -x

    sudo yum install -y gcc openssl-devel libyaml-devel libffi-devel readline-devel zlib-devel gdbm-devel ncurses-devel

    if [ -d ${HOME}/.rbenv ]; then
        pushd ${HOME}/.rbenv
        git pull
        popd
    else
        git clone https://github.com/rbenv/rbenv.git ${HOME}/.rbenv
    fi

    if ! grep "PATH=.*\.rbenv/bin" ${HOME}/.bash_profile; then
        echo 'export PATH="${HOME}/.rbenv/bin:$PATH"' >> ~/.bash_profile
    fi

    if ! grep "rbenv init -" ${HOME}/.bash_profile; then
        echo 'eval "$(rbenv init -)"' >> ~/.bash_profile
    fi

    export PATH="${HOME}/.rbenv/bin:${PATH}"
    eval "$(rbenv init -)"

    if [ -d ${HOME}/.rbenv/plugins/ruby-build ]; then
        pushd ${HOME}/.rbenv/plugins/ruby-build
        git pull
        popd
    else
        git clone https://github.com/rbenv/ruby-build.git ${HOME}/.rbenv/plugins/ruby-build
    fi

    rbenv install -s #{BOX_RUBY_VERSION}
    rbenv global #{BOX_RUBY_VERSION}
    rbenv rehash
    gem install --conservative toml docker-api mogilefs-client
    rbenv rehash
  _NACHOS

  # TODO: Need to install rbenv / ruby-build and install Ruby, rake, and the toml gem.
end

Vagrant.configure(2) do |config|
  config.vm.define "centos6", primary: true do |c6|
    c6.vm.box = "bento/centos-6.7"
    common_centos_config(c6)
  end

  # Due to the GLIBC 2.14 memcpy / memmove scandal, we should probably
  # just build on CentOS 6, so that we don't get the Bad
  # Symbol. However, supermin doesn't compile on CentOS 6, so the base
  # Docker image is built with CentOS 7.
  config.vm.define "centos7", autostart: false do |c7|
    c7.vm.box = "bento/centos-7.1"
    common_centos_config(c7)
  end
end
