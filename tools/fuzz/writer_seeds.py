"""Original writer-v1 instruction seeds, with invented encoded frame bytes."""
import argparse
from pathlib import Path


def header():
    out = bytearray(20)
    for index, value in {0: 2, 4: 2, 5: 3, 6: 3, 7: 3, 8: 15, 9: 3}.items():
        out[index] = value
    return out


def block(ticks=0, track=0, flags=0, count=3, lace=0, padding=0,
          duration=0, references=0, payload=b'abcd', tag=0):
    if len(payload) % 4 or len(payload) > 1020:
        raise ValueError('writer seed payload must be a multiple of four through 1020')
    return (bytes([tag, track]) + ticks.to_bytes(8, 'little', signed=True)
            + bytes([flags, count, lace, padding & 255, duration, references, len(payload)//4])
            + payload)


def generate(root):
    root.mkdir(parents=True, exist_ok=True)

    def save(name, data):
        path = root / name
        if path.exists() and path.read_bytes() != data:
            raise ValueError('existing writer seed differs: ' + str(path))
        path.write_bytes(data)

    base = header()
    sequence = block() + block(20) + bytes([1]) + block(40) + bytes([2])
    save('writer-basic', base + sequence)
    save('writer-empty', b'')
    for name, lace in [('fixed', 1), ('xiph', 2), ('ebml', 3)]:
        for padding in [-25, 0, 25]:
            save(f'writer-{name}-pad-{padding}', base + block(flags=8, lace=lace,
                 padding=padding, duration=60) + block(60) + bytes([2]))
        for count in [0, 1, 8, 9, 10]:
            save(f'writer-{name}-count-{count}', base + block(flags=8, count=count,
                 lace=lace) + bytes([2]))
        save(f'writer-{name}-large', base + block(flags=8, count=3, lace=lace,
             payload=b'abcd'*150) + bytes([2]))
    for flags in [1, 2, 4, 8|64, 8|32]:
        save(f'writer-flags-{flags}', base + block(flags=flags, lace=1, duration=60) + bytes([2]))
    for refs in [64, 96, 128, 160, 192, 224]:
        save(f'writer-refs-{refs}', base + block(flags=1, references=refs) + bytes([2]))
    for index, values in {0:[0,1,2,3,6,10,18,50],4:[0,1,3],5:[0,1,2],6:[0,1,2],7:[0,1,2],8:[0,4,8],9:[0,1,2,7],11:[1,2],12:[1,2,4]}.items():
        for value in values:
            control=header();control[index]=value
            save(f'writer-config-{index}-{value}', control + sequence)
    control=header();control[0]|=16
    save('writer-two-tracks',control+block()+block(20,track=1)+block(40)+bytes([2]))
    for kind in range(1,7):
        control=header();control[0]|=16;control[17]=kind
        save(f'writer-codec-{kind}',control+block()+block(20,track=1)+bytes([2]))
    for scale in [0,1,2,3]:
        control=header();control[0]=scale;control[16]=1
        save(f'writer-u64-clock-{scale}',control+block(ticks=-1)+bytes([2]))
    save('writer-unknown-track',base+block(track=2)+block()+bytes([2]))
    save('writer-nonmonotonic',base+block(40)+block(20)+bytes([2]))
    save('writer-negative',base+block(-20)+block()+bytes([2]))
    for flags in [0,16]:
        save(f'writer-extreme-{flags}',base+block(tag=128,flags=flags)+block()+bytes([2]))
    save('writer-abort-buffered',base+block()+bytes([3]))
    for mode in range(1,14):
        for at in [1,3,4,5,6]:
            control=header();control[1]=mode;control[2]=at-1
            save(f'writer-fault-{mode}-{at}',control+sequence)
    for chunk in [1,7,31]:
        control=header();control[3]=chunk
        save(f'writer-short-{chunk}',control+sequence)


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('out',type=Path)
    generate(parser.parse_args().out)
