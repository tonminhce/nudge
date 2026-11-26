import { createHash, createHmac, generateKeyPairSync, sign } from 'node:crypto';

const b64url = (value) => Buffer.from(value).toString('base64url');

const { privateKey, publicKey } = generateKeyPairSync('ec', { namedCurve: 'P-256' });

const privatePem = privateKey.export({ type: 'pkcs8', format: 'pem' }).toString();
const publicPem = publicKey.export({ type: 'spki', format: 'pem' }).toString();

const appSecret = 'test-app-secret';
const workspaceId = '11111111-2222-3333-4444-555555555555';

const legacySecret = createHash('sha256')
  .update(appSecret + workspaceId + 'ACCESS')
  .digest('hex');

const hsHeader = b64url(JSON.stringify({ alg: 'HS256', typ: 'JWT' }));
const hsPayload = b64url(
  JSON.stringify({
    sub: workspaceId,
    userId: workspaceId,
    workspaceId,
    type: 'ACCESS',
    exp: 4102444800,
  }),
);
const hsInput = `${hsHeader}.${hsPayload}`;
const hsSignature = createHmac('sha256', legacySecret).update(hsInput).digest('base64url');

const esHeader = b64url(JSON.stringify({ alg: 'ES256', typ: 'JWT' }));
const esPayload = b64url(JSON.stringify({ sub: 'user-1', type: 'ACCESS', exp: 4102444800 }));
const esInput = `${esHeader}.${esPayload}`;
const esSignature = sign('sha256', Buffer.from(esInput), {
  key: privatePem,
  dsaEncoding: 'ieee-p1363',
}).toString('base64url');

console.log(
  JSON.stringify(
    {
      appSecret,
      workspaceId,
      legacySecret,
      publicPem,
      hsToken: `${hsInput}.${hsSignature}`,
      esToken: `${esInput}.${esSignature}`,
    },
    null,
    1,
  ),
);
