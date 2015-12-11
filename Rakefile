require "docker"
require "mogilefs"
require "pathname"
require "rake/packagetask"
require "rake/tasklib"
require "toml"

# noop.

class CargoBuildTask < Rake::TaskLib
  attr_accessor :name, :project, :binary, :lib, :release, :sources

  def initialize(name, project, binary: nil, lib: false, release: false, sources: project.sources)
    self.name = name
    self.project = project
    self.binary = binary
    self.lib = lib
    self.release = release
    self.sources = sources
    define
  end

  def define
    CargoBuildTask::Task.define_task(project, binary, lib, release, sources, name, [ :verbose ] => sources) do |t, args|
      args.with_defaults(:verbose => false)
      project.cd_dir
      build(args.verbose)
      project.cd_root
    end
  end

  class Task < Rake::Task
    attr_accessor :project, :binary, :lib, :release, :sources

    def self.define_task(project, binary, lib, release, sources, *args, &block)
      Rake.application.define_task(self, *args, &block).tap do |task|
        task.project = project
        task.binary = binary
        task.lib = lib
        task.release = release
        task.sources = sources
        task.add_description(task.description)
      end
    end

    def description
      "Build #{building_what} in #{project.name} -- #{profile}"
    end

    def building_what
      if binary
        binary
      elsif lib
        "lib#{project.name}.rlib"
      else
        "all"
      end
    end

    def timestamp
      sources.map { |f| File.mtime(f) rescue Time.at(0) }.max
    end

    def profile
      release ? "release" : "debug"
    end

    def output_timestamp
      target_profile_path = project.target_path(full: true).join(profile)

      if binary
        File.mtime(target_profile_path.join(binary)) rescue Time.at(0)
      elsif lib
        File.mtime(target_profile_path.join("lib#{project.name}.rlib")) rescue Time.at(0)
      elsif !target_profile_path.exist?
        Time.at(0)
      else
        min_time = target_profile_path.children
          .select { |c| c.exist? && c.file? }
          .map { |c| c.mtime }
          .min
        min_time || Time.at(0)
      end
    end

    def needed?
      timestamp > output_timestamp
    end
  end

  def build(verbose)
    cmdline = [ "cargo", "build" ]

    if binary
      cmdline += [ "--bin", binary ]
    end

    # if package
    #   cmdline += [ "-p", package ]
    # end

    if lib
      cmdline << "--lib"
    end

    if release
      cmdline << "--release"
    end

    if verbose
      cmdline << "--verbose"
    end

    sh(*cmdline)
  end
end

class RustProjectTask < Rake::TaskLib
  @@projects = {}
  def self.[](*args)
    @@projects.send(:[], *args)
  end

  def self.[]=(*args)
    @@projects.send(:[]=, *args)
  end

  attr_accessor :path

  def initialize(path = ".")
    self.path = Pathname.new(path)
    @should_define = RustProjectTask[path].nil?
    RustProjectTask[path] = self

    # Look for the subprojects before defining tasks.
    subprojects

    if @should_define
      define_build_tasks
      define_test_tasks
      define_clean_tasks
    end
  end

  def root_path
    @root_path ||= Pathname.new(__FILE__).dirname.expand_path
  end

  def full_path
    @full_path ||= root_path.join(path).expand_path
  end

  def relative_path
    @relative_path = full_path.relative_path_from(root_path)
  end

  def in_subpath?
    if @in_subpath.nil?
      @in_subpath = root_path != full_path
    end
    @in_subpath
  end

  def cargo_toml_path(full: true)
    if full
      @full_cargo_toml_path ||= full_path.join("Cargo.toml")
    else
      @relative_cargo_toml_path ||= relative_path.join("Cargo.toml")
    end
  end

  def src_path(full: false)
    if full
      @full_src_path ||= full_path.join("src")
    else
      @relative_src_path ||= relative_path.join("src")
    end
  end

  def target_path(full: false)
    if full
      @full_target_path ||= full_path.join("target")
    else
      @relative_target_path ||= relative_path.join("target")
    end
  end

  def sources
    @sources ||= FileList[
      relative_path.join("Cargo.*").to_s,
      src_path.join("**", "*.rs").to_s,
    ].exclude { |f| !File.exist?(f) }
  end

  def recursive_sources
    if @recursive_sources.nil?
      @recursive_sources = sources.dup
      subprojects.each do |p|
        @recursive_sources += p.recursive_sources
      end
    end
    @recursive_sources
  end

  def binary_sources
    @binary_sources ||= FileList[
      src_path.join("main.rs").to_s,
      src_path.join("bin", "*.rs").to_s,
    ].exclude { |f| !File.exist?(f) }
  end

  def lib_sources
    @lib_sources ||= FileList[
      src_path.join("lib.rs").to_s
    ].exclude { |f| !File.exist?(f) }
  end

  def cargo_data
    @cargo_data ||= TOML.load_file(cargo_toml_path)
  end

  def name
    @name ||= cargo_data["package"]["name"]
  end

  def subprojects
    @subprojects ||= cargo_data["dependencies"].map do |name, opts|
      if opts.is_a?(Hash) && opts["path"]
        RustProjectTask.new(relative_path.join(opts["path"]).to_s)
      end
    end.compact
  end

  def cd_dir
    cd full_path.to_s if in_subpath?
  end

  def cd_root
    cd root_path.to_s if in_subpath?
  end

  def do_in_dir
    cd_dir
    yield
    cd_root
  end

  def project_namespace
    if in_subpath?
      string_as_task(relative_path.to_s)
    else
      string_as_task(name)
    end
  end

  def string_as_task(str)
    str.gsub(/[^A-Za-z0-9]/, "_").squeeze("_")
  end

  def in_project_namespace
    namespace project_namespace do
      yield
    end
  end

  def build_task_name(artifact, release: false)
    [ project_namespace,
      "build",
      string_as_task(artifact),
      release ? "release" : "debug"
    ].join(":")
  end

  def define_build_tasks
    in_project_namespace do
      namespace :build do
        CargoBuildTask.new(:debug, self, sources: recursive_sources)
        CargoBuildTask.new(:release, self, sources: recursive_sources, release: true)

        binary_sources.each do |bin_source|
          bin_srcdir = File.basename(File.dirname(bin_source))
          bin_name = File.basename(bin_source, ".rs")
          if bin_srcdir == "src" && bin_name == "main"
            namespace string_as_task(name) do
              CargoBuildTask.new(:debug, self, binary: name, sources: recursive_sources)
              CargoBuildTask.new(:release, self, binary: name, sources: recursive_sources, release: true)
            end
          elsif bin_srcdir == "bin"
            namespace string_as_task(bin_name) do
              CargoBuildTask.new(:debug, self, binary: bin_name, sources: recursive_sources)
              CargoBuildTask.new(:release, self, binary: bin_name, sources: recursive_sources, release: true)
            end
          end
        end

        lib_sources.each do |lib_source|
          namespace :lib do
            CargoBuildTask.new(:debug, self, lib: true, sources: recursive_sources)
            CargoBuildTask.new(:release, self, lib: true, sources: recursive_sources, release: true)
          end
        end
      end
    end

    debug_path = target_path(full: false).join("debug")
    release_path = target_path(full: false).join("release")

    binary_sources.each do |bin_source|
      bin_srcdir = File.basename(File.dirname(bin_source))
      bin_name = File.basename(bin_source, ".rs")

      if bin_srcdir == "src" && bin_name == "main"
        file debug_path.join(name), [ :verbose ] => [ build_task_name(name) ]
        file release_path.join(name), [ :verbose ] => [ build_task_name(name, release: true) ]
      elsif bin_srcdir == "bin"
        file debug_path.join(bin_name), [ :verbose ] => [ build_task_name(bin_name) ]
        file release_path.join(bin_name), [ :verbose ] => [ build_task_name(bin_name, release: true) ]
      end
    end

    lib_sources.each do |lib_source|
      lib_name = "lib#{name}.rlib"
      file debug_path.join(lib_name), [ :verbose ] => [ build_task_name("lib") ]
      file release_path.join(lib_name), [ :verbose ] => [ build_task_name("lib", release: true) ]
    end
  end

  def define_test_tasks
    in_project_namespace do
      desc "run tests for #{name}"
      task :test, [ :verbose ] do |t, args|
        args.with_defaults(verbose: false)
        cmdline = [ "cargo", "test" ]
        if args[:verbose]
          cmdline << "--verbose"
        end
        do_in_dir { sh(*cmdline) }
      end
    end
  end

  def define_clean_tasks
    in_project_namespace do
      desc "Clean #{name}"
      task :clean, [ :with_deps, :verbose ] do |t, args|
        args.with_defaults(verbose: false, with_deps: false)

        cmdline = [ "cargo", "clean" ]

        if args.verbose
          cmdline << "--verbose"
        end

        if args.with_deps
          do_in_dir { sh(*cmdline) }
        else
          do_in_dir do
            ([ name ] + subprojects.map(&:name)).each do |package|
              cmdline2 = cmdline + [ "-p", package ]
              sh(*cmdline2)
            end
          end
        end
      end
    end
  end
end

def cargo_data(path = ".")
  @cargo_data ||= {}
  @cargo_data[path] ||= RustProjectTask[path].cargo_data
end

def git_revision
  File.read("git-revision").strip
end

def release_filename
  metadata = cargo_data
  package = metadata["package"]
  if RUBY_PLATFORM == "x86_64-linux"
    triple = "x86_64-unknown-linux-gnu"
  elsif RUBY_PLATFORM == "x86_64-linux-gnu"
      triple = "x86_64-unknown-linux-gnu"
  elsif RUBY_PLATFORM =~ /universal.x86_64-darwin1[45]/
    triple = "x86_64-apple-darwin"
  else
    raise "Unknown triple for platform #{RUBY_PLATFORM.inspect}"
  end

  "filament-#{package["version"]}-#{triple}.tar.gz"
end

RustProjectTask.new

namespace :docker do
  desc "Set up the shell environment for testing with the real mogilefs cluster."
  task :env do
    ENV["COMPOSE_PROJECT_NAME"] = "filament"
    ENV["FILAMENT_TEST_DOMAIN"] = "test_domain"
    ENV["FILAMENT_TEST_CLASS"] = "test_class"
    ENV["FILAMENT_TEST_DB_USER"] = "mogile"
    ENV["FILAMENT_TEST_DB_PASS"] = "blarg"
    ENV["FILAMENT_TEST_DB_NAME"] = "mogilefs"
  end

  desc "Start the docker containers."
  task :start => [ :env ] do
    sh "docker-compose", "-f", "test/containers/docker-compose.yml", "up", "-d"
  end

  desc "Stop the docker containers."
  task :stop => [ :env ] do
    sh "docker-compose", "-f", "test/containers/docker-compose.yml", "stop"
  end

  desc "Delete the docker containers."
  task :clean => [ :stop ] do
    sh "docker-compose", "-f", "test/containers/docker-compose.yml", "rm", "-v", "--force"
  end

  desc "Make sure the docker mogilefs infrastructure is properly set up with hosts, devices, domains, and classes."
  task :init => [ :start ] do
    docker_host = ENV["DOCKER_HOST"] || "tcp://127.0.0.1:2375"
    docker_ip = docker_host.match(/^tcp:\/\/(.*):\d+$/).captures.first

    # Figure out the DB IP / port.
    db = Docker::Container.all("all" => 1).find { |c| c.info["Names"].include?("/filament_db_1") }
    db_port = db.info["Ports"].find { |p| p["PrivatePort"] == 3306 }["PublicPort"]
    db_addr = "#{docker_ip}:#{db_port}"
    puts "db_addr = #{db_addr.inspect}"
    ENV["FILAMENT_TEST_DB_HOST"] = db_addr

    # Figure out the tracker IP / port.
    tracker = Docker::Container.all("all" => 1).find { |c| c.info["Names"].include?("/filament_mogilefsd_1") }
    tracker_port = tracker.info["Ports"].find { |p| p["PrivatePort"] == 7001 }["PublicPort"]
    tracker_addr = "#{docker_ip}:#{tracker_port}"
    puts "tracker_addr = #{tracker_addr.inspect}"
    ENV["FILAMENT_TEST_TRACKERS"] = tracker_addr

    # Figure out the storage server IP / port.
    storage = Docker::Container.all("all" => 1).find { |c| c.info["Names"].include?("/filament_storage_1_1") }
    storage_port = storage.info["Ports"].find { |p| p["PrivatePort"] == 7500 }["PublicPort"]
    storage_addr = "#{docker_ip}:#{storage_port}"
    puts "storage_addr = #{storage_addr.inspect}"

    # Create our client objects.
    mogadm = MogileFS::Admin.new(hosts: [ tracker_addr ])
    # mogcl = MogileFS::MogileFS.new(hosts: [ tracker_addr ], domain: "test_domain")

    # Wait until the tracker is ready to handle requests.
    loop do
      begin
        mogadm.get_domains
        break
      rescue
        puts "#{$!.message} (#{$!.class}), retrying..."
        sleep 2
      end
    end

    # Create / update the host for the storage server.
    storage_host = mogadm.get_hosts.find { |h| h["hostname"] == "storage_1" }
    if storage_host.nil?
      mogadm.create_host("storage_1", ip: docker_ip, port: storage_port, status: "alive")
    else
      mogadm.update_host("storage_1", ip: docker_ip, port: storage_port, status: "alive")
    end

    # Create / update the device on the storage server.
    storage_device = mogadm.get_devices.find { |d| d["devid"] == 1 }
    if storage_device.nil?
      mogadm.create_device("storage_1", 1, status: "alive")
    else
      mogadm.change_device_state("storage_1", 1, "alive")
    end

    # Create the test domain.
    test_domain = mogadm.get_domains["test_domain"]
    if test_domain.nil?
      mogadm.create_domain("test_domain")
      test_domain = mogadm.get_domains["test_domain"]
    end

    # Make sure the default class has a mindevcount of 1.
    if test_domain["default"]["mindevcount"] != 1
      mogadm.update_class("test_domain", "default", mindevcount: 1)
    end

    # Create the test class.
    if test_domain["test_class"].nil?
      mogadm.create_class("test_domain", "test_class", mindevcount: 1)
      test_domain = mogadm.get_domains["test_domain"]
    end

    # Make sure the test class has a mindevcount of 1.
    if test_domain["test_class"]["mindevcount"] != 1
      mogadm.update_class("test_domain", "test_class", mindevcount: 1)
    end
  end
end

desc "The dist directory"
directory "dist"

desc "The release tarball"
file File.join("dist", release_filename) => [ "target/release/filament", "target/release/filament-cli", "dist" ] do
  sh "tar", "-czf", File.join("dist", release_filename), "-C", "target/release", "filament", "filament-cli"
end

desc "Compile the source, and put the release tarball in dist"
task :package => [ File.join("dist", release_filename) ]

desc "Build debug builds of all the things in all the subdirs (you probably don't need to do this)"
task :build, [ :verbose ] => [ "filament:build:debug", "client:build:debug", "common:build:debug", "server:build:debug" ]

desc "Build release builds of all the things in all the subdirs (you probably really don't need to do this)"
task :build_release, [ :verbose ] => [ "filament:build:release", "client:build:release", "common:build:release", "server:build:release" ]

desc "Clean the main crate and the sub-crates"
task :clean, [ :with_deps, :verbose ] => [ "filament:clean", "client:clean", "common:clean", "server:clean" ]

namespace :test do
  desc "Run the tests for all the sub-crates, skipping things that require a real MogileFS cluster."
  task :unit, [ :verbose ] => [ :build, :test ]

  desc "Run the tests for all the sub-crates, using a real MogileFS running in Docker."
  task :integration, [ :verbose ] => [ "docker:env", "docker:init", :build, :test ]
end

task :test, [ :verbose ] => [ "filament:test", "client:test", "common:test", "server:test" ]

desc "Build the docs"
task :doc do
  sh "cargo", "doc"
end

task :default, [ :verbose ] => [ "test:unit" ]
