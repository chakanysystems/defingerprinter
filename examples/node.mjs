import { execFileSync } from 'node:child_process';

const request = {
  op: 'redact', language: 'en', text: 'Contact alice@example.com',
};
const output = execFileSync(process.argv[2], [], {
  input: JSON.stringify(request) + '\n',
  encoding: 'utf8',
});
const response = JSON.parse(output);
if (!response.ok) throw new Error(response.reason);
console.log(response.text); // Contact [EMAIL_1]
