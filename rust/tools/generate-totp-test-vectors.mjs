import { createHash, createHmac } from 'node:crypto';

const BASE32_ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';

const decodeBase32 = (secret) => {
  let bits = 0;
  let value = 0;
  const bytes = [];

  for (const char of secret.replace(/=+$/, '')) {
    const index = BASE32_ALPHABET.indexOf(char.toUpperCase());

    if (index < 0) {
      throw new Error(`invalid base32 character: ${char}`);
    }

    value = (value << 5) | index;
    bits += 5;

    if (bits >= 8) {
      bytes.push((value >>> (bits - 8)) & 0xff);
      bits -= 8;
    }
  }

  return Buffer.from(bytes);
};

const encodeBase32 = (bytes) => {
  let bits = 0;
  let value = 0;
  let output = '';

  for (const byte of bytes) {
    value = (value << 8) | byte;
    bits += 8;

    while (bits >= 5) {
      output += BASE32_ALPHABET[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }

  if (bits > 0) {
    output += BASE32_ALPHABET[(value << (5 - bits)) & 31];
  }

  return output;
};

const totpPadSecret = (hexSecret, minLengthBytes) => {
  const currentLength = hexSecret.length;

  if (currentLength < minLengthBytes) {
    const repeated = new Array(minLengthBytes - currentLength + 1).join(
      hexSecret,
    );

    return Buffer.from(repeated, 'hex').subarray(0, minLengthBytes);
  }

  return Buffer.from(hexSecret, 'hex');
};

const totpCode = (
  base32Secret,
  epochMs,
  { step = 30, digits = 6, algorithm = 'sha1' } = {},
) => {
  const counter = Math.floor(epochMs / 1000 / step);
  const hexSecret = decodeBase32(base32Secret).toString('hex');
  const key = totpPadSecret(hexSecret, 20);
  const counterBuffer = Buffer.alloc(8);

  counterBuffer.writeBigUInt64BE(BigInt(counter));

  const digest = createHmac(algorithm, key).update(counterBuffer).digest();
  const offset = digest[digest.length - 1] & 0xf;
  const binary =
    ((digest[offset] & 0x7f) << 24) |
    ((digest[offset + 1] & 0xff) << 16) |
    ((digest[offset + 2] & 0xff) << 8) |
    (digest[offset + 3] & 0xff);

  return String(binary % Math.pow(10, digits)).padStart(digits, '0');
};

const encodeURIComponent = (value) =>
  Array.from(Buffer.from(value, 'utf8'))
    .map((byte) => {
      const char = String.fromCharCode(byte);

      return /[A-Za-z0-9\-_.!~*'()]/.test(char)
        ? char
        : `%${byte.toString(16).toUpperCase()}`;
    })
    .join('');

const otpauthUri = (accountName, issuer, secret) =>
  `otpauth://totp/${encodeURIComponent(issuer)}:${encodeURIComponent(
    accountName,
  )}?secret=${secret}&period=30&digits=6&algorithm=SHA1&issuer=${encodeURIComponent(issuer)}`;

const hashRecoveryCode = (code) =>
  createHash('sha256')
    .update(code.toUpperCase().replace(/[^0-9A-Z]/g, ''))
    .digest('hex');

const RFC_SECRET = 'GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ';
const RFC_SECRET_BYTES = decodeBase32(RFC_SECRET);

if (RFC_SECRET_BYTES.toString('binary') !== '12345678901234567890') {
  throw new Error('the RFC 6238 base32 secret does not decode to the RFC secret');
}

const RFC_TIMES = [59, 1111111109, 1111111111, 1234567890, 2000000000, 20000000000];
const rfc8 = RFC_TIMES.map((seconds) => totpCode(RFC_SECRET, seconds * 1000, { digits: 8 }));
const rfc6 = RFC_TIMES.map((seconds) => totpCode(RFC_SECRET, seconds * 1000, { digits: 6 }));

const TEN_BYTE_SECRET = encodeBase32(Buffer.from('1234567890', 'ascii'));
const SHORT_SECRET = encodeBase32(Buffer.from('12345678', 'ascii'));
const LONG_SECRET = encodeBase32(
  Buffer.from('1234567890123456789012345678901234', 'ascii'),
);

const paddedShortKey = totpPadSecret(decodeBase32(SHORT_SECRET).toString('hex'), 20);

const baseSeconds = 1111111109;
const windowSamples = {
  at: totpCode(TEN_BYTE_SECRET, baseSeconds * 1000),
  minusOneStep: totpCode(TEN_BYTE_SECRET, (baseSeconds - 30) * 1000),
  minusTwoSteps: totpCode(TEN_BYTE_SECRET, (baseSeconds - 60) * 1000),
  plusOneStep: totpCode(TEN_BYTE_SECRET, (baseSeconds + 30) * 1000),
  plusTwoSteps: totpCode(TEN_BYTE_SECRET, (baseSeconds + 60) * 1000),
};

const rawBytes = Buffer.from('a3b1c2d3e4f506172839', 'hex');
const maskedBytes = Buffer.from(rawBytes.map((byte) => byte & 0x7f));
const generatedSecret = encodeBase32(maskedBytes);

const vectors = {
  rfc6238: {
    secret: RFC_SECRET,
    times: RFC_TIMES,
    eightDigits: rfc8,
    sixDigits: rfc6,
  },
  tenByte: {
    secret: TEN_BYTE_SECRET,
    at1234567890: totpCode(TEN_BYTE_SECRET, 1234567890000),
    at1234567845: totpCode(TEN_BYTE_SECRET, 1234567845000),
    at1234567915: totpCode(TEN_BYTE_SECRET, 1234567915000),
  },
  shortSecret: {
    secret: SHORT_SECRET,
    paddedKeyHex: paddedShortKey.toString('hex'),
    at59: totpCode(SHORT_SECRET, 59 * 1000),
  },
  longSecret: {
    secret: LONG_SECRET,
    length: LONG_SECRET.length,
    at1234567890: totpCode(LONG_SECRET, 1234567890000),
  },
  window: { secret: TEN_BYTE_SECRET, baseSeconds, ...windowSamples },
  generatedSecret: {
    rawHex: rawBytes.toString('hex'),
    maskedHex: maskedBytes.toString('hex'),
    secret: generatedSecret,
    codeAt59: totpCode(generatedSecret, 59 * 1000),
  },
  uri: otpauthUri('totp-vectors@nudge.test', 'Twenty - Nudge HQ', TEN_BYTE_SECRET),
  uriPlainIssuer: otpauthUri('totp-vectors@nudge.test', 'Twenty', TEN_BYTE_SECRET),
  recoveryCodes: {
    plain: 'ABCDE-FGHIJ-KLMNP-QRSTV',
    hash: hashRecoveryCode('ABCDE-FGHIJ-KLMNP-QRSTV'),
    lowercaseWithNoise: hashRecoveryCode('abcde-fghij-klmnp-qrstv!@# '),
    uppercased: hashRecoveryCode('abcde-fghij-klmnp-qrstv'),
  },
};

console.log(JSON.stringify(vectors, null, 1));
