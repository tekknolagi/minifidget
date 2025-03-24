import numpy as np

prog = []

with open('prospero.vm') as f:
    for line in f:
        if line.startswith('#'):
            continue
        prog.append(line.split())

with_gc = []
seen = set()

for (out, op, *args) in reversed(prog):
    if op != "const" and with_gc:
        for arg in args:
            if arg not in seen:
                with_gc.append(("_", "gc", arg))
        seen.update(args)
    with_gc.append((out, op, *args))

prog = with_gc[::-1]

side = 1024
space = np.linspace(-1, 1, side)
(x, y) = np.meshgrid(space, space)
v = {}

for (out, op, *args) in prog:
    match op:
        case "gc": del v[args[0]]
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
