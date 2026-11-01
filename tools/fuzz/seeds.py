"""Original container/block fuzz seeds; invented packet bytes, no decoding claim."""
import argparse
from pathlib import Path


def size(value):
    width = next(width for width in range(1, 9) if value < (1 << (7 * width)) - 1)
    return ((1 << (7 * width)) | value).to_bytes(width, 'big')


def element(identifier, data, unknown=False):
    encoded = identifier.to_bytes((identifier.bit_length() + 7) // 8, 'big')
    return encoded + (b'\xff' if unknown else size(len(data))) + data


def number(identifier, value):
    return element(identifier, value.to_bytes(8, 'big'))


def block(track, timestamp, flags, payload):
    return bytes([0x80 | track]) + timestamp.to_bytes(2, 'big', signed=True) + bytes([flags]) + payload


def opus_head():
    return b'OpusHead' + bytes([1, 1, 56, 1]) + (48000).to_bytes(4, 'little') + bytes(3)


def document(unknown=False, opus=False, scale=1_000_000, cluster_time=0, cues=True, dense=False):
    header = element(0x1a45dfa3, element(0x4282, b'webm') + number(0x4287, 4) + number(0x4285, 2))
    info = element(0x1549a966, element(0x4d80, b'fuzz seed') + element(0x5741, b'original')
                   + number(0x2ad7b1, scale))
    audio = element(0xe1, element(0xb5, bytes.fromhex('40e7700000000000')) + number(0x9f, 1))
    private = opus_head() if opus else b'original'
    track = (number(0xd7, 1) + number(0x73c5, 1) + number(0x83, 2)
             + element(0x86, b'A_OPUS' if opus else b'A_TEST') + element(0x63a2, private)
             + number(0x56aa, 6_500_000 if opus else 7) + number(0x56bb, 80_000_000)
             + number(0x23e383, 20_000_000) + audio)
    segment = info + element(0x1654ae6b, element(0xae, track))
    positions = []
    for timestamp in ([0] if dense else [cluster_time, 100]):
        offset = len(segment)
        time = number(0xe7, timestamp)
        simple = element(0xa3, block(1, 0, 0x80, b'encoded-first'))
        group = element(0xa0, element(0xa1, block(1, 20, 4, b'\x01aabb'))
                        + number(0x9b, 40) + element(0x75a2, b'\x09'))
        if dense:
            simple = element(0xa3, b'\x81\x00\x00\x04\x1f') * 1000
            group = b''
        segment += element(0x1f43b675, time + simple + group, unknown)
        positions.append((timestamp, offset, len(time)))
    if cues:
        cue_offset = len(segment)
        payload = b''.join(
            element(0xbb, number(0xb3, timestamp) + element(
                0xb7, number(0xf7, 1) + number(0xf1, offset) + number(0xf0, relative)))
            for timestamp, offset, relative in positions
        )
        segment += element(0x1c53bb6b, payload)
        segment += element(0x114d9b74, element(
            0x4dbb, element(0x53ab, bytes.fromhex('1c53bb6b')) + number(0x53ac, cue_offset)))
    return header + element(0x18538067, segment, unknown)


def generate(root):
    root.mkdir(parents=True, exist_ok=True)

    def save(name, mode, data):
        timestamp = (150_000_000).to_bytes(16, 'little', signed=True) if mode == 3 else b''
        value = bytes([mode]) + timestamp + data
        path = root / name
        if path.exists() and path.read_bytes() != value:
            raise ValueError('existing seed differs: ' + str(path))
        path.write_bytes(value)

    scalars = [
        ('empty', b''), ('zero', bytes(16)), ('ones', b'\xff' * 16),
        ('ebml', bytes.fromhex('1a45dfa3')), ('plain', block(1, -32768, 0x80, b'abc')),
        ('fixed', block(1, 32767, 4, b'\x01aabb')),
        ('xiph', block(1, 1, 2, b'\x01\x02aabb')),
        ('lace-ebml', block(1, 1, 6, b'\x02\x82\xbfaabbcc')), ('opus', opus_head()),
    ]
    for name, data in scalars:
        save('scalar-' + name, 0, data)
    for unknown in [False, True]:
        child = element(0xa1, element(0xa2, b'ab') + element(0xa3, b'payload'), unknown)
        save('hierarchy-' + str(unknown), 1, element(
            0xa0, child + element(0xa4, b'metadata'), unknown))
        for opus in [False, True]:
            for cues in [False, True]:
                data = document(unknown, opus, cues=cues)
                for mode in [2, 3]:
                    save(f'document-{unknown}-{opus}-{cues}-{mode}', mode, data)
    for scale, timestamp in [(2**64 - 1, 2**64 - 1), (1, 2**64 - 1), (2**63, 2**63)]:
        data = document(scale=scale, cluster_time=timestamp)
        for mode in [2, 3]:
            save(f'timestamp-{scale}-{timestamp}-{mode}', mode, data)
    for mode in [2, 3]:
        save('dense-empty-laces-' + str(mode), mode, document(cues=False, dense=True))
    save('source-actions', 4, bytes(range(256)))
    save('source-budget', 4, bytes([2, 0]) * 50)
    save('source-revision', 4, bytes([0, 0, 18, 0, 0, 0, 2, 4, 0, 0]))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    generate(parser.parse_args().output)
