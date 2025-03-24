import numpy as np

with open('prospero.vm') as f:
    text = f.read().strip()

side = 512
space = np.linspace(-1, 1, side)
(x, y) = np.meshgrid(space, space)
v = {}

for line in text.split('\n'):
    if line.startswith('#'):
        continue
    [out, op, *args] = line.split()
    match op:
        case "var-x": v[out] = x
        case "var-y": v[out] = -y
        case "const": v[out] = float(args[0])
        case "add": v[out] = v[args[0]] + v[args[1]]
        case "sub": v[out] = v[args[0]] - v[args[1]]
        case "mul": v[out] = v[args[0]] * v[args[1]]
        case "max": v[out] = np.maximum(v[args[0]], v[args[1]])
        case "min": v[out] = np.minimum(v[args[0]], v[args[1]])
        case "neg": v[out] = -v[args[0]]
        case "square": v[out] = v[args[0]] * v[args[0]]
        case "sqrt": v[out] = np.sqrt(v[args[0]])
        case _: raise Exception(f"unknown opcode '{op}'")
out = v[out]

out = np.sign(-out)
out = (out + 1) / 2 * 255
out = out.astype(np.uint8)
with open('out.ppm', 'wb') as f:
    f.write(f'P5\n{side} {side}\n255\n'.encode())
    f.write(out.tobytes())
