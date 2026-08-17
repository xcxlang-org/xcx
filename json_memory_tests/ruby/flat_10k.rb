require 'json'
require 'time'

payload_tbl = []
10000.times do |i|
  payload_tbl << { id: i, name: "User_#{i}", active: true }
end

t0 = Time.now

data = payload_tbl
raw_str = JSON.generate(data)
File.write('flat_10000_temp.json', raw_str)

read_str = ""
if File.exist?('flat_10000_temp.json')
  read_str = File.read('flat_10000_temp.json')
  File.delete('flat_10000_temp.json')
end

parsed = JSON.parse(read_str)
count = parsed.length
mid_elem = parsed[5000]
name_mid = mid_elem["name"]

t1 = Time.now
json_ms = (t1 - t0) * 1000.0
puts "json_elapsed_ms: #{'%.3f' % json_ms}"
