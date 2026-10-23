// Hearth Calculator's arithmetic (ui/engine.js). Run: node --test tests/engine.test.js
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const C = require('../ui/engine.js');
const { D } = C;
const ev = (s, opts) => C.evaluate(s.split(' ').map(t => (/^-?[\d.]+(e[+-]?\d+)?$/i.test(t) ? D.parse(t) : t)), opts).format();

test('decimal arithmetic has no float surprises', () => {
  assert.equal(D.parse('0.1').add('0.2').format(), '0.3');
  assert.equal(D.parse('1.1').mul('1.1').format(), '1.21');
  assert.equal(D.parse('0.3').sub('0.1').format(), '0.2');
  assert.equal(D.parse('19.99').mul(3).format(), '59.97');
  assert.equal(D.parse('10').div(4).format(), '2.5');
  assert.equal(D.parse('1').div(3).mul(3).format(), '1');
  assert.equal(D.parse('2').div(3).format(), '0.66666666666666666666666666666667');
  assert.equal(D.parse('100000000000000000000').add(1).format(), '100000000000000000001');
  assert.equal(D.parse('-5.50').format(), '-5.5');
  assert.equal(D.parse('1e-7').format(), '0.0000001');
  assert.equal(D.parse('1.5e-12').format(), '1.5e-12');
  assert.equal(D.parse('123456789').mul('987654321').format({ group: true }), '121,932,631,112,635,269');
});

test('big and tiny numbers', () => {
  assert.equal(C.pow(2, 64).format(), '18446744073709551616');
  assert.equal(C.pow(10, 30).format(), '1e+30');
  assert.equal(C.factorial(25).format(), '1.5511210043330985984e+25');
  assert.equal(C.factorial(20).format(), '2432902008176640000');
  assert.throws(() => C.factorial(-1));
});

test('division by zero and invalid input are errors, not NaN', () => {
  assert.throws(() => D.parse('1').div(0), /divide by zero/);
  assert.throws(() => D.parse('0').div(0), /undefined/);
  assert.throws(() => C.sqrt(-4), /Invalid/);
  assert.throws(() => C.unary('ln', 0), /Invalid/);
});

test('roots and powers', () => {
  assert.equal(C.sqrt(2).format(), '1.4142135623730950488016887242097');
  assert.equal(C.sqrt(144).format(), '12');
  assert.equal(C.sqrt('0.0001').format(), '0.01');
  assert.equal(C.root(27, 3).format(), '3');
  assert.equal(C.root(-8, 3).format(), '-2');
  assert.equal(C.pow(-8, D.parse('1').div(3)).format(), '-2');
  assert.equal(C.pow(2, -2).format(), '0.25');
  assert.equal(C.pow(9, '0.5').format(), '3');
});

test('trigonometry is exact at the usual angles', () => {
  assert.equal(C.trig('sin', 30, 'deg').format(), '0.5');
  assert.equal(C.trig('cos', 60, 'deg').format(), '0.5');
  assert.equal(C.trig('sin', 180, 'deg').format(), '0');
  assert.equal(C.trig('cos', 90, 'deg').format(), '0');
  assert.equal(C.trig('tan', 45, 'deg').format(), '1');
  assert.throws(() => C.trig('tan', 90, 'deg'));
  assert.equal(C.trig('sin', 100, 'grad').format(), '1');
  assert.equal(C.trig('asin', 1, 'deg').format(), '90');
  assert.equal(C.trig('sin', C.PI.div(6), 'rad').format(), '0.5');
  assert.equal(C.unary('log', 1000).format(), '3');
  assert.equal(C.unary('ln', C.E).format(), '1');
});

test('precedence: scientific respects it, standard goes left to right', () => {
  assert.equal(ev('2 + 3 * 4'), '14');
  assert.equal(ev('2 + 3 * 4', { precedence: false }), '20');
  assert.equal(ev('( 2 + 3 ) * 4'), '20');
  assert.equal(ev('2 ^ 3 ^ 2'), '512');
  assert.equal(ev('10 mod 3'), '1');
  assert.equal(ev('-7 mod 3'), '2');
  assert.equal(ev('1.5 E 3'), '1500');
  assert.throws(() => ev('2 +'));
});

test('programmer mode: widths, bases and bit operations', () => {
  const b = 32;
  assert.equal(C.toBase(255n, 16, b), 'FF');
  assert.equal(C.toBase(-1n, 16, 8), 'FF');
  assert.equal(C.toBase(-1n, 2, 8), '11111111');
  assert.equal(C.toBase(0xFFn, 10, 8), '-1');
  assert.equal(C.wrap(2147483648n, 32), -2147483648n);
  assert.equal(C.parseBase('FF', 16), 255n);
  assert.equal(C.parseBase('1010', 2), 10n);
  assert.equal(C.pevaluate([12n, 'AND', 10n], b), 8n);
  assert.equal(C.pevaluate([12n, 'OR', 3n, 'XOR', 1n], b), 14n);
  assert.equal(C.pevaluate([1n, '<<', 4n, '+', 1n], b), 32n);
  assert.equal(C.pevaluate([-16n, '>>', 2n], b), -4n);
  assert.equal(C.pevaluate([7n, '/', 2n], b), 3n);
  assert.equal(C.pbinary('rol', 0x80000001n, 1n, 32), 3n);
  assert.equal(C.pbinary('ror', 1n, 1n, 8), -128n);
  assert.equal(C.pbinary('NAND', 0n, 0n, 8), -1n);
  assert.equal(C.groupBase('11110000', 2), '1111 0000');
  assert.equal(C.groupBase('1234567', 10), '1,234,567');
  assert.throws(() => C.parseBase('G', 16));
});

test('unit conversion', () => {
  assert.equal(C.convert('Length', 1, 'in', 'cm').format(), '2.54');
  assert.equal(C.convert('Length', 1, 'mi', 'km').format(), '1.609344');
  assert.equal(C.convert('Temperature', 100, 'C', 'F').format(), '212');
  assert.equal(C.convert('Temperature', -40, 'F', 'C').format(), '-40');
  assert.equal(C.convert('Temperature', 0, 'C', 'K').format(), '273.15');
  assert.equal(C.convert('Data size', 1, 'GiB', 'MB').format(), '1073.741824');
  assert.equal(C.convert('Data size', 8, 'bit', 'B').format(), '1');
  assert.equal(C.convert('Time', 1, 'd', 'min').format(), '1440');
  assert.equal(C.convert('Mass', 1, 'lb', 'g').format(), '453.59237');
  assert.equal(C.convert('Speed', 100, 'km/h', 'm/s').format(), '27.777777777777777777777777777778');
});
