# ChaCha20 reference output

Expected keystream bytes in `cases.json` are copied from
[RFC 8439](https://www.rfc-editor.org/rfc/rfc8439.html), *ChaCha20 and
Poly1305 for IETF Protocols* (Nir & Langley, June 2018). They were not
produced by this engine.

The two cases use the same 256-bit key and the same initial block
counter, but **different 96-bit nonces**. §2.4.2 is the ChaCha20 cipher
example, not the AEAD construction in §2.8.2.

## Source

https://www.rfc-editor.org/rfc/rfc8439.html

## `rfc8439_2_3_2_block` — RFC 8439 §2.3.2

Test vector for the ChaCha20 block function.

Inputs (RFC §2.3.2):

- Key = `00:01:02:03:04:05:06:07:08:09:0a:0b:0c:0d:0e:0f:10:11:12:13:14:15:16:17:18:19:1a:1b:1c:1d:1e:1f`
- Nonce = `00:00:00:09:00:00:00:4a:00:00:00:00`
- Block Count = 1

Serialized 64-byte block (RFC §2.3.2 dump):

```text
000  10 f1 e7 e4 d1 3b 59 15 50 0f dd 1f a3 20 71 c4
016  c7 d1 f4 c7 33 c0 68 03 04 22 aa 9a c3 d4 6c 4e
032  d2 82 64 46 07 9f aa 09 14 c2 d7 05 d9 8b 02 a2
048  b5 12 9c d1 de 16 4e b9 cb d0 83 e8 a2 50 3c 4e
```

`cases.json` stores this dump as a lowercase hex string with no
separators. Comparison is exact; there is no tolerance.

## `rfc8439_2_4_2_keystream` — RFC 8439 §2.4.2

Example and test vector for the ChaCha20 cipher (not AEAD).

Inputs (RFC §2.4.2):

- Key = `00:01:02:03:04:05:06:07:08:09:0a:0b:0c:0d:0e:0f:10:11:12:13:14:15:16:17:18:19:1a:1b:1c:1d:1e:1f`
- Nonce = `00:00:00:00:00:00:00:4a:00:00:00:00`
- Initial Counter = 1
- Plaintext = `Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.` (114 octets)

Keystream figure (RFC §2.4.2), 114 bytes:

```text
22:4f:51:f3:40:1b:d9:e1:2f:de:27:6f:b8:63:1d:ed:8c:13:1f:82:3d:2c:06
e2:7e:4f:ca:ec:9e:f3:cf:78:8a:3b:0a:a3:72:60:0a:92:b5:79:74:cd:ed:2b
93:34:79:4c:ba:40:c6:3e:34:cd:ea:21:2c:4c:f0:7d:41:b7:69:a6:74:9f:3f
63:0f:41:22:ca:fe:28:ec:4d:c4:7e:26:d4:34:6d:70:b9:8c:73:f3:e9:c5:3a
c4:0c:59:45:39:8b:6e:da:1a:83:2c:89:c1:67:ea:cd:90:1d:7e:2b:f3:63
```

`cases.json` stores this figure as a lowercase hex string with no
separators. Comparison is exact; there is no tolerance.

The RFC also publishes the corresponding ciphertext. XOR of that
ciphertext with the plaintext recovers the same 114-byte keystream; the
fixture uses the Keystream figure as the normative expected value.
