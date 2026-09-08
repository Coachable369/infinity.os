#!/usr/bin/env ruby
require 'json'

# Verify two real QEMU framebuffer artifacts differ only inside the saved progress frame.
# Keep the pointer inside that frame during capture: cursor restoration is a separate compositor operation.
abort 'usage: installer-progress-damage-test.rb BEFORE.ppm AFTER.ppm TEMPLATE.infinityui' unless ARGV.length == 3
images = ARGV.first(2).map do |path|
  magic, dimensions, maximum, pixels = File.binread(path).split("\n", 4)
  raise 'invalid framebuffer artifact' unless magic == 'P6' && maximum == '255'
  width, height = dimensions.split.map(&:to_i)
  raise 'truncated framebuffer' unless pixels.bytesize == width * height * 3
  [width, height, pixels]
end
width, height, before = images[0]
other_width, other_height, after = images[1]
raise 'framebuffer dimensions changed' unless [width, height] == [other_width, other_height]
document = JSON.parse(File.read(ARGV[2]))
element = document.fetch('screens').find { |screen| screen.fetch('id') == 8 }
  .fetch('elements').find { |layer| layer.fetch('role') == 19 }
raise 'progress is hidden' if element.fetch('hidden')
frame = element.fetch('frame')
left = frame.fetch('x') * width / 1000
top = frame.fetch('y') * height / 1000
right = left + frame.fetch('width') * width / 1000
bottom = top + frame.fetch('height') * height / 1000
changed = 0
height.times do |y|
  width.times do |x|
    offset = (y * width + x) * 3
    next if before.byteslice(offset, 3) == after.byteslice(offset, 3)
    raise "pixel escaped progress bounds: #{x},#{y}" unless x >= left && x < right && y >= top && y < bottom
    changed += 1
  end
end
raise 'no observable progress update' if changed.zero?
puts JSON.generate(width: width, height: height, changed_pixels: changed, outside_progress: 0)
