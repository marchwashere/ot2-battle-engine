'use strict';
// Build-time only: read the user's local cooked assets through the existing extractor.
const path = require('node:path');
const root = path.resolve(__dirname, '../..');
const assets = require(path.join(root, 'OT2BattleEngine/tools/assets.cjs'));
const extract = require(path.join(root, 'OT2BattleEngine/tools/extract.cjs'));
const cleanName = s => s.replace(/_\d+_[A-F0-9]{32}$/, '');
const fields = ps => Object.fromEntries(ps.map(p => [cleanName(p.name.text), value(p.value)]));
function value(v) {
  if (v === null || typeof v !== 'object') return v;
  if (Array.isArray(v)) return v.map(value);
  if (v.properties) return fields(v.properties);
  if (v.elements) return v.elements.map(value);
  if (v.kind === 'enum') return v.value.text;
  if (v.kind === 'float32') return v.value;
  if (v.text !== undefined) return v.text;
  return v;
}
function table(name) {
  const p = assets.loadPackage(path.join(assets.CONTENT, name + '.uasset'));
  const ex = p.exports.find(e => assets.objectPath(p, e.classIndex) === '/Script/Engine.DataTable');
  if (!ex) throw new Error('No table in ' + name);
  return { source: '/Game/' + name, sha256: assets.hash(p.b), rows: extract.table(p, ex).rows.map(r => ({id: r.name.text, ...fields(r.properties)})) };
}
module.exports = { root, assets, extract, fields, value, table };
