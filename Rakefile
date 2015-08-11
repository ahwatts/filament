require "toml"

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
  else
    raise "Unknown triple for platform #{RUBY_PLATFORM.inspect}"
  end

  "mogilefsd-#{package["version"]}-#{triple}.tar.gz"
end

def dist_dir
  "dist"
end

task :compile do
  sh "cargo", "build", "--release"
end

directory dist_dir

file binary_file => [ :compile ]

file release_file => [ binary_file, dist_dir ] do
  dir, file = File.dirname(binary_file), File.basename(binary_file)
  sh "tar -czf #{File.join(dist_dir, release_file)} -C #{dir} #{file}"
end

task :package => [ release_file ]
