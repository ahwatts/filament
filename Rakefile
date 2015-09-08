require "toml"

def root_dir
  File.expand_path(File.dirname(__FILE__))
end

def subdirs
  %w{ . client common server }
end

def in_each_subdir
  failed_subdirs = []
  subdirs.each do |dir|
    cd File.expand_path(File.join(root_dir, dir))
    begin
      yield
    rescue
      failed_subdirs << dir
      STDERR.puts "Error running in subdir %s: %s (%p)" %
        [ dir, $!.message, $!.class, ]
    end
  end
  cd root_dir

  unless failed_subdirs.empty?
    raise "Some subdirs failed: #{failed_subdirs.inspect}"
  end
end

def cargo_data
  TOML.load_file("Cargo.toml")
end

def git_revision
  File.read("git-revision").strip
end

def binary_file
  "target/release/mogilefsd"
end

def release_file
  metadata = cargo_data
  package = metadata["package"]
  if RUBY_PLATFORM == "x86_64-linux"
    triple = "x86_64-unknown-linux-gnu"
  elsif RUBY_PLATFORM == "universal.x86_64-darwin14"
    triple = "x86_64-apple-darwin"
  else
    raise "Unknown triple for platform #{RUBY_PLATFORM.inspect}"
  end

  "mogilefsd-#{package["version"]}-#{triple}.tar.gz"
end

def dist_dir
  "dist"
end

desc "Compile a release build (e.g. cargo build --release)."
task :compile do
  sh "cargo", "build", "--release"
end

desc "The dist directory"
directory dist_dir

desc "The built release executable"
file binary_file => [ :compile ]

desc "The release tarball"
file release_file => [ binary_file, dist_dir ] do
  dir, file = File.dirname(binary_file), File.basename(binary_file)
  sh "tar -czf #{File.join(dist_dir, release_file)} -C #{dir} #{file}"
end

desc "Compile the source, and put the release tarball in dist"
task :package => [ release_file ]

desc "Build all the sub-crates (you probably don't need to do this)"
task :build do
  in_each_subdir { sh "cargo", "build" }
end

desc "Clean the sub-crates"
task :clean do
  in_each_subdir { sh "cargo", "clean" }
end

desc "Run the tests for all the sub-crates"
task :test do
  in_each_subdir { sh "cargo", "test" }
end

desc "Build the docs"
task :doc do
  in_each_subdir { sh "cargo", "doc" }
end
