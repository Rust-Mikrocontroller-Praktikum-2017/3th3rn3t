import math

samples = []

def sin440(x):
    return int(math.sin(440/(2*math.pi) * x / 48000) * 2**15 + 2**15)

for i in range(48000):
    samples.append(sin440(i))

output = ""
for sample in samples:
    output += "0x{0:04x},".format(sample)

print("[" + output + "]")

