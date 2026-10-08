# frozen_string_literal: true

require "minitest/autorun"
require "net/http"
require "open3"
require "socket"

class PreviewLocalTest < Minitest::Test
  STARTUP_TIMEOUT = 300

  def test_preview_serves_site_routes_and_binds_only_to_loopback
    script = File.expand_path("../bin/preview-local", __dir__)
    assert File.executable?(script), "expected executable bin/preview-local"

    root = File.expand_path("..", __dir__)
    lockfile = File.join(root, "Gemfile.lock")
    lock_before = File.binread(lockfile)
    port = available_port
    routes = ["/", "/publications/", "/assets/css/main.css"]

    Open3.popen2e({ "PORT" => port.to_s }, script, pgroup: true) do |stdin, output, server|
      stdin.close
      output_thread = Thread.new { output.each_line.to_a.join }
      begin
        responses = wait_for_routes(port, routes, server, output_thread)

        assert_equal "200", responses.fetch("/").code
        assert_equal "200", responses.fetch("/publications/").code
        assert_includes responses.fetch("/publications/").body, "Publications"
        assert_equal "200", responses.fetch("/assets/css/main.css").code
        assert_match %r{text/css}, responses.fetch("/assets/css/main.css")["content-type"]

        listeners, result = Open3.capture2e("lsof", "-nP", "-iTCP:#{port}", "-sTCP:LISTEN")
        assert result.success?, "expected lsof to inspect preview listener: #{listeners}"
        assert_includes listeners, "127.0.0.1:#{port} (LISTEN)"
        refute_match(/(?:\*|0\.0\.0\.0):#{port}/, listeners)

        assert_equal lock_before, File.binread(lockfile)
      ensure
        Process.kill("TERM", -server.pid) if server.alive?
        server.join
        output_thread.join
      end
    end
  end

  private

  def available_port
    socket = TCPServer.new("127.0.0.1", 0)
    socket.addr[1]
  ensure
    socket&.close
  end

  def wait_for_routes(port, routes, server, output_thread)
    deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + STARTUP_TIMEOUT

    loop do
      responses = routes.to_h { |path| [path, get(path, port)] }
      return responses if responses.values.all?
      raise "preview server exited before serving requests:\n#{output_thread.value}" unless server.alive?
      raise "preview server did not start within #{STARTUP_TIMEOUT}s" if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline

      sleep 0.25
    end
  end

  def get(path, port)
    http = Net::HTTP.new("127.0.0.1", port, nil)
    http.open_timeout = 0.5
    http.read_timeout = 5
    http.get(path)
  rescue EOFError, IOError, Net::OpenTimeout, Net::ReadTimeout, SocketError, SystemCallError
    nil
  end
end
