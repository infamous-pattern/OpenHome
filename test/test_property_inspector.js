'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const root = path.resolve(__dirname, '..');
const source = fs.readFileSync(path.join(root, 'assets/propertyInspector/openhome.js'), 'utf8');
const context = vm.createContext({
  console,
  URL,
  window: { addEventListener() {} },
  setTimeout,
  clearTimeout
});
vm.runInContext(source, context, { filename: 'openhome.js' });

function normalise(value) {
  return vm.runInContext(`normaliseHomebridgeUrl(${JSON.stringify(value)})`, context);
}

assert.equal(normalise('homebridge.local:8581'), 'http://homebridge.local:8581');
assert.equal(normalise('http://10.52.10.19:8581/'), 'http://10.52.10.19:8581');
assert.equal(normalise('https://homebridge.example.test/base/api'), 'https://homebridge.example.test/base');
assert.throws(() => normalise('http://10.52.10.19:99999'), /address or port is invalid/);
assert.throws(() => normalise('ftp://homebridge.local:8581'), /must use http/);
assert.throws(() => normalise('http://homebridge.local:8581/?token=bad'), /query string/);
assert.throws(() => normalise(''), /Enter the Homebridge UI address/);


function characteristicType(value) {
  return vm.runInContext(`characteristicTypeOf(${JSON.stringify(value)})`, context);
}

assert.equal(characteristicType({ type: 'On' }), 'On');
assert.equal(characteristicType({ characteristicType: 'On' }), 'On');
assert.equal(characteristicType({ type: 'On', characteristicType: 'LegacyOn' }), 'LegacyOn');
assert.equal(characteristicType({}), '');


function cycleValues(value) {
  return JSON.parse(vm.runInContext(`JSON.stringify(parseCycleValues(${JSON.stringify(value)}))`, context));
}

assert.deepEqual(cycleValues('75, 25, 50, 25, 100'), [25, 50, 75, 100]);
assert.deepEqual(cycleValues(''), [25, 50, 75, 100]);
assert.deepEqual(cycleValues('10; 20 30'), [10, 20, 30]);

console.log('Property inspector URL, characteristic mapping, and brightness preset tests passed.');
