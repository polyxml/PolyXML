const test = require('node:test');
const assert = require('node:assert');

test('PolyXML JavaScript bindings', (t) => {
  let polyxml;
  try {
    polyxml = require('../index.js');
  } catch (err) {
    // If native addon not yet compiled locally, skip with notice
    t.diagnostic('Native addon not compiled in current environment; skipping live execution test.');
    return;
  }

  assert.strictEqual(typeof polyxml.version(), 'string');

  const schema = {
    name: 'Sensor',
    fields: [
      { name: 'id', xmlName: 'id', kind: 'attribute', scalarType: 'int' },
      { name: 'name', xmlName: 'name', kind: 'element', scalarType: 'string' },
      { name: 'reading', xmlName: 'reading', kind: 'element', scalarType: 'float' },
      { name: 'calibrated', xmlName: 'calibrated', kind: 'element', scalarType: 'bool' },
    ],
  };

  const xml = '<Sensor id="101"><name>Barometric Altimeter</name><reading>1013.25</reading><calibrated>true</calibrated></Sensor>';
  const val = polyxml.deserialize(xml, schema);

  assert.strictEqual(val.id, 101);
  assert.strictEqual(val.name, 'Barometric Altimeter');
  assert.strictEqual(val.reading, 1013.25);
  assert.strictEqual(val.calibrated, true);

  // Test serialization
  const outBytes = polyxml.serialize('Sensor', val, schema, 2);
  assert.ok(outBytes instanceof Uint8Array || Buffer.isBuffer(outBytes));
  const outXml = Buffer.from(outBytes).toString('utf-8');
  assert.ok(outXml.includes('id="101"'));
  assert.ok(outXml.includes('<name>Barometric Altimeter</name>'));
  assert.ok(outXml.includes('<reading>1013.25</reading>'));
  assert.ok(outXml.includes('<calibrated>true</calibrated>'));

  // Test namespaced schema & serialization
  const orderSchema = {
    name: 'Order',
    namespace: 'https://example.com/orders',
    fields: [
      { name: 'id', xmlName: 'id', kind: 'attribute', scalarType: 'int' },
      { name: 'item', xmlName: 'item', kind: 'element', scalarType: 'string', namespace: 'https://example.com/items' },
    ],
  };

  const orderXml = '<ns0:Order xmlns:ns0="https://example.com/orders" xmlns:ns1="https://example.com/items" id="777"><ns1:item>NodeGadget</ns1:item></ns0:Order>';
  const orderVal = polyxml.deserialize(orderXml, orderSchema);
  assert.strictEqual(orderVal.id, 777);
  assert.strictEqual(orderVal.item, 'NodeGadget');

  const orderOut = polyxml.serialize('Order', orderVal, orderSchema, 0, true, {
    ord: 'https://example.com/orders',
    itm: 'https://example.com/items',
  });
  const orderOutStr = Buffer.from(orderOut).toString('utf-8');
  assert.ok(orderOutStr.includes('xmlns:ord="https://example.com/orders"'));
  assert.ok(orderOutStr.includes('xmlns:itm="https://example.com/items"'));
  assert.ok(orderOutStr.includes('<ord:Order'));
  assert.ok(orderOutStr.includes('<itm:item>NodeGadget</itm:item>'));

  // Test real-world fixture conformance: atom_feed.xml
  const fs = require('node:fs');
  const path = require('node:path');
  const atomPath = path.resolve(__dirname, '../../../tests/fixtures/samples/atom_feed.xml');
  if (fs.existsSync(atomPath)) {
    const atomXml = fs.readFileSync(atomPath, 'utf-8');
    const atomSchema = {
      name: 'feed',
      namespace: 'http://www.w3.org/2005/Atom',
      fields: [
        { name: 'title', xmlName: 'title', kind: 'element', scalarType: 'string' },
        { name: 'id', xmlName: 'id', kind: 'element', scalarType: 'string' },
      ],
    };
    const atomVal = polyxml.deserialize(atomXml, atomSchema);
    assert.strictEqual(atomVal.title, 'PolyXML Engineering Updates');

    const atomOut = polyxml.serialize('feed', atomVal, atomSchema, 2, true, {
      '': 'http://www.w3.org/2005/Atom',
    });
    const atomOutStr = Buffer.from(atomOut).toString('utf-8');
    assert.ok(atomOutStr.includes('xmlns="http://www.w3.org/2005/Atom"'));
    assert.ok(atomOutStr.includes('<title>PolyXML Engineering Updates</title>'));
  }

  // Test nested schemas, lists, required fields, and polymorphic xsi:type variants
  const addressSchema = {
    name: 'Address',
    fields: [
      { name: 'city', xmlName: 'city', kind: 'element', scalarType: 'string', required: true },
      { name: 'zip', xmlName: 'zip', kind: 'element', scalarType: 'int' },
    ],
  };
  const carSchema = {
    name: 'Car',
    xmlName: 'Car',
    fields: [
      { name: 'id', xmlName: 'id', kind: 'element', scalarType: 'string', required: true },
      { name: 'doors', xmlName: 'doors', kind: 'element', scalarType: 'int' },
    ],
  };
  const truckSchema = {
    name: 'Truck',
    xmlName: 'Truck',
    fields: [
      { name: 'id', xmlName: 'id', kind: 'element', scalarType: 'string', required: true },
      { name: 'payload', xmlName: 'payload', kind: 'element', scalarType: 'string' },
    ],
  };
  const vehicleSchema = {
    name: 'Vehicle',
    xmlName: 'Vehicle',
    isAbstract: true,
    variants: [carSchema, truckSchema],
    fields: [
      { name: 'id', xmlName: 'id', kind: 'element', scalarType: 'string', required: true },
    ],
  };
  const shipmentSchema = {
    name: 'Shipment',
    fields: [
      { name: 'destination', xmlName: 'destination', kind: 'element', nestedSchema: addressSchema, required: true },
      { name: 'tags', xmlName: 'tag', kind: 'element', scalarType: 'string', isList: true },
      { name: 'vehicles', xmlName: 'vehicle', kind: 'element', nestedSchema: vehicleSchema, isList: true },
    ],
  };

  const shipmentXml = '<Shipment xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">'
    + '<destination><city>Austin</city><zip>78701</zip></destination>'
    + '<tag>express</tag><tag>insured</tag>'
    + '<vehicle xsi:type="Car"><id>c1</id><doors>4</doors></vehicle>'
    + '<vehicle xsi:type="Truck"><id>t1</id><payload>ore</payload></vehicle>'
    + '</Shipment>';
  const shipment = polyxml.deserialize(shipmentXml, shipmentSchema);
  assert.deepStrictEqual(shipment, {
    destination: { city: 'Austin', zip: 78701 },
    tags: ['express', 'insured'],
    vehicles: [
      { '@xsi:type': 'Car', id: 'c1', doors: 4 },
      { '@xsi:type': 'Truck', id: 't1', payload: 'ore' },
    ],
  });

  const shipmentOut = Buffer.from(polyxml.serialize('Shipment', shipment, shipmentSchema)).toString('utf-8');
  assert.ok(shipmentOut.includes('<destination><city>Austin</city><zip>78701</zip></destination>'));
  assert.ok(shipmentOut.includes('<tag>express</tag><tag>insured</tag>'));
  assert.ok(shipmentOut.includes('xsi:type="Car"'));
  assert.ok(shipmentOut.includes('<doors>4</doors>'));
  assert.ok(shipmentOut.includes('xsi:type="Truck"'));
  assert.ok(shipmentOut.includes('<payload>ore</payload>'));

  const shipmentCopy = polyxml.deserialize(shipmentOut, shipmentSchema);
  assert.deepStrictEqual(shipmentCopy, shipment);
});
