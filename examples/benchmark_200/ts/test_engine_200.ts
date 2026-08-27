import assert from "node:assert/strict";
import test from "node:test";

import * as engine from "./engine_200.ts";

type BinaryFunction = (first: number, second: number) => number;
type NamedFunction = readonly [name: string, fn: BinaryFunction];

const mathFunctions: readonly NamedFunction[] = [
  ["mathFunc001", engine.mathFunc001],
  ["mathFunc002", engine.mathFunc002],
  ["mathFunc003", engine.mathFunc003],
  ["mathFunc004", engine.mathFunc004],
  ["mathFunc005", engine.mathFunc005],
  ["mathFunc006", engine.mathFunc006],
  ["mathFunc007", engine.mathFunc007],
  ["mathFunc008", engine.mathFunc008],
  ["mathFunc009", engine.mathFunc009],
  ["mathFunc010", engine.mathFunc010],
  ["mathFunc011", engine.mathFunc011],
  ["mathFunc012", engine.mathFunc012],
  ["mathFunc013", engine.mathFunc013],
  ["mathFunc014", engine.mathFunc014],
  ["mathFunc015", engine.mathFunc015],
  ["mathFunc016", engine.mathFunc016],
  ["mathFunc017", engine.mathFunc017],
  ["mathFunc018", engine.mathFunc018],
  ["mathFunc019", engine.mathFunc019],
  ["mathFunc020", engine.mathFunc020],
  ["mathFunc021", engine.mathFunc021],
  ["mathFunc022", engine.mathFunc022],
  ["mathFunc023", engine.mathFunc023],
  ["mathFunc024", engine.mathFunc024],
  ["mathFunc025", engine.mathFunc025],
  ["mathFunc026", engine.mathFunc026],
  ["mathFunc027", engine.mathFunc027],
  ["mathFunc028", engine.mathFunc028],
  ["mathFunc029", engine.mathFunc029],
  ["mathFunc030", engine.mathFunc030],
  ["mathFunc031", engine.mathFunc031],
  ["mathFunc032", engine.mathFunc032],
  ["mathFunc033", engine.mathFunc033],
  ["mathFunc034", engine.mathFunc034],
  ["mathFunc035", engine.mathFunc035],
  ["mathFunc036", engine.mathFunc036],
  ["mathFunc037", engine.mathFunc037],
  ["mathFunc038", engine.mathFunc038],
  ["mathFunc039", engine.mathFunc039],
  ["mathFunc040", engine.mathFunc040],
];

const statFunctions: readonly NamedFunction[] = [
  ["statFunc041", engine.statFunc041],
  ["statFunc042", engine.statFunc042],
  ["statFunc043", engine.statFunc043],
  ["statFunc044", engine.statFunc044],
  ["statFunc045", engine.statFunc045],
  ["statFunc046", engine.statFunc046],
  ["statFunc047", engine.statFunc047],
  ["statFunc048", engine.statFunc048],
  ["statFunc049", engine.statFunc049],
  ["statFunc050", engine.statFunc050],
  ["statFunc051", engine.statFunc051],
  ["statFunc052", engine.statFunc052],
  ["statFunc053", engine.statFunc053],
  ["statFunc054", engine.statFunc054],
  ["statFunc055", engine.statFunc055],
  ["statFunc056", engine.statFunc056],
  ["statFunc057", engine.statFunc057],
  ["statFunc058", engine.statFunc058],
  ["statFunc059", engine.statFunc059],
  ["statFunc060", engine.statFunc060],
  ["statFunc061", engine.statFunc061],
  ["statFunc062", engine.statFunc062],
  ["statFunc063", engine.statFunc063],
  ["statFunc064", engine.statFunc064],
  ["statFunc065", engine.statFunc065],
  ["statFunc066", engine.statFunc066],
  ["statFunc067", engine.statFunc067],
  ["statFunc068", engine.statFunc068],
  ["statFunc069", engine.statFunc069],
  ["statFunc070", engine.statFunc070],
  ["statFunc071", engine.statFunc071],
  ["statFunc072", engine.statFunc072],
  ["statFunc073", engine.statFunc073],
  ["statFunc074", engine.statFunc074],
  ["statFunc075", engine.statFunc075],
  ["statFunc076", engine.statFunc076],
  ["statFunc077", engine.statFunc077],
  ["statFunc078", engine.statFunc078],
  ["statFunc079", engine.statFunc079],
  ["statFunc080", engine.statFunc080],
  ["statFunc081", engine.statFunc081],
  ["statFunc082", engine.statFunc082],
  ["statFunc083", engine.statFunc083],
  ["statFunc084", engine.statFunc084],
  ["statFunc085", engine.statFunc085],
  ["statFunc086", engine.statFunc086],
  ["statFunc087", engine.statFunc087],
  ["statFunc088", engine.statFunc088],
  ["statFunc089", engine.statFunc089],
  ["statFunc090", engine.statFunc090],
];

const combatFunctions: readonly NamedFunction[] = [
  ["combatFunc091", engine.combatFunc091],
  ["combatFunc092", engine.combatFunc092],
  ["combatFunc093", engine.combatFunc093],
  ["combatFunc094", engine.combatFunc094],
  ["combatFunc095", engine.combatFunc095],
  ["combatFunc096", engine.combatFunc096],
  ["combatFunc097", engine.combatFunc097],
  ["combatFunc098", engine.combatFunc098],
  ["combatFunc099", engine.combatFunc099],
  ["combatFunc100", engine.combatFunc100],
  ["combatFunc101", engine.combatFunc101],
  ["combatFunc102", engine.combatFunc102],
  ["combatFunc103", engine.combatFunc103],
  ["combatFunc104", engine.combatFunc104],
  ["combatFunc105", engine.combatFunc105],
  ["combatFunc106", engine.combatFunc106],
  ["combatFunc107", engine.combatFunc107],
  ["combatFunc108", engine.combatFunc108],
  ["combatFunc109", engine.combatFunc109],
  ["combatFunc110", engine.combatFunc110],
  ["combatFunc111", engine.combatFunc111],
  ["combatFunc112", engine.combatFunc112],
  ["combatFunc113", engine.combatFunc113],
  ["combatFunc114", engine.combatFunc114],
  ["combatFunc115", engine.combatFunc115],
  ["combatFunc116", engine.combatFunc116],
  ["combatFunc117", engine.combatFunc117],
  ["combatFunc118", engine.combatFunc118],
  ["combatFunc119", engine.combatFunc119],
  ["combatFunc120", engine.combatFunc120],
  ["combatFunc121", engine.combatFunc121],
  ["combatFunc122", engine.combatFunc122],
  ["combatFunc123", engine.combatFunc123],
  ["combatFunc124", engine.combatFunc124],
  ["combatFunc125", engine.combatFunc125],
  ["combatFunc126", engine.combatFunc126],
  ["combatFunc127", engine.combatFunc127],
  ["combatFunc128", engine.combatFunc128],
  ["combatFunc129", engine.combatFunc129],
  ["combatFunc130", engine.combatFunc130],
  ["combatFunc131", engine.combatFunc131],
  ["combatFunc132", engine.combatFunc132],
  ["combatFunc133", engine.combatFunc133],
  ["combatFunc134", engine.combatFunc134],
  ["combatFunc135", engine.combatFunc135],
  ["combatFunc136", engine.combatFunc136],
  ["combatFunc137", engine.combatFunc137],
  ["combatFunc138", engine.combatFunc138],
  ["combatFunc139", engine.combatFunc139],
  ["combatFunc140", engine.combatFunc140],
];

const worldFunctions: readonly NamedFunction[] = [
  ["worldFunc141", engine.worldFunc141],
  ["worldFunc142", engine.worldFunc142],
  ["worldFunc143", engine.worldFunc143],
  ["worldFunc144", engine.worldFunc144],
  ["worldFunc145", engine.worldFunc145],
  ["worldFunc146", engine.worldFunc146],
  ["worldFunc147", engine.worldFunc147],
  ["worldFunc148", engine.worldFunc148],
  ["worldFunc149", engine.worldFunc149],
  ["worldFunc150", engine.worldFunc150],
  ["worldFunc151", engine.worldFunc151],
  ["worldFunc152", engine.worldFunc152],
  ["worldFunc153", engine.worldFunc153],
  ["worldFunc154", engine.worldFunc154],
  ["worldFunc155", engine.worldFunc155],
  ["worldFunc156", engine.worldFunc156],
  ["worldFunc157", engine.worldFunc157],
  ["worldFunc158", engine.worldFunc158],
  ["worldFunc159", engine.worldFunc159],
  ["worldFunc160", engine.worldFunc160],
  ["worldFunc161", engine.worldFunc161],
  ["worldFunc162", engine.worldFunc162],
  ["worldFunc163", engine.worldFunc163],
  ["worldFunc164", engine.worldFunc164],
  ["worldFunc165", engine.worldFunc165],
  ["worldFunc166", engine.worldFunc166],
  ["worldFunc167", engine.worldFunc167],
  ["worldFunc168", engine.worldFunc168],
  ["worldFunc169", engine.worldFunc169],
  ["worldFunc170", engine.worldFunc170],
  ["worldFunc171", engine.worldFunc171],
  ["worldFunc172", engine.worldFunc172],
  ["worldFunc173", engine.worldFunc173],
  ["worldFunc174", engine.worldFunc174],
  ["worldFunc175", engine.worldFunc175],
  ["worldFunc176", engine.worldFunc176],
  ["worldFunc177", engine.worldFunc177],
  ["worldFunc178", engine.worldFunc178],
  ["worldFunc179", engine.worldFunc179],
  ["worldFunc180", engine.worldFunc180],
];

const engineFunctions: readonly NamedFunction[] = [
  ["engineFunc181", engine.engineFunc181],
  ["engineFunc182", engine.engineFunc182],
  ["engineFunc183", engine.engineFunc183],
  ["engineFunc184", engine.engineFunc184],
  ["engineFunc185", engine.engineFunc185],
  ["engineFunc186", engine.engineFunc186],
  ["engineFunc187", engine.engineFunc187],
  ["engineFunc188", engine.engineFunc188],
  ["engineFunc189", engine.engineFunc189],
  ["engineFunc190", engine.engineFunc190],
  ["engineFunc191", engine.engineFunc191],
  ["engineFunc192", engine.engineFunc192],
  ["engineFunc193", engine.engineFunc193],
  ["engineFunc194", engine.engineFunc194],
  ["engineFunc195", engine.engineFunc195],
  ["engineFunc196", engine.engineFunc196],
  ["engineFunc197", engine.engineFunc197],
  ["engineFunc198", engine.engineFunc198],
  ["engineFunc199", engine.engineFunc199],
  ["engineFunc200", engine.engineFunc200],
];

test("exports exactly the 200 numbered engine functions", () => {
  const expectedNames = [
    ...mathFunctions,
    ...statFunctions,
    ...combatFunctions,
    ...worldFunctions,
    ...engineFunctions,
  ].map(([name]) => name).sort();

  assert.equal(expectedNames.length, 200);
  assert.deepEqual(Object.keys(engine).sort(), expectedNames);
});

for (const [name, fn] of mathFunctions) {
  test(name, () => {
    assert.equal(fn(7, 5), 12);
    assert.equal(fn(0, 0), 0);
    assert.throws(() => fn(-1, 0), RangeError);
    assert.throws(() => fn(0, -1), RangeError);
  });
}

for (const [name, fn] of statFunctions) {
  test(name, () => {
    assert.equal(fn(10, 4), 14);
    assert.equal(fn(0, 0), 0);
    assert.throws(() => fn(-1, 0), RangeError);
    assert.throws(() => fn(0, -1), RangeError);
  });
}

for (const [name, fn] of combatFunctions) {
  test(name, () => {
    assert.equal(fn(12, 5), 7);
    assert.equal(fn(5, 5), 1);
    assert.equal(fn(3, 8), 1);
    assert.throws(() => fn(-1, 0), RangeError);
    assert.throws(() => fn(0, -1), RangeError);
  });
}

for (const [name, fn] of worldFunctions) {
  test(name, () => {
    assert.equal(fn(10, 4), 14);
    assert.equal(fn(10, 0), 11);
    assert.throws(() => fn(-1, 0), RangeError);
    assert.throws(() => fn(0, -1), RangeError);
  });
}

for (const [name, fn] of engineFunctions) {
  test(name, () => {
    assert.equal(fn(17, 5), 2);
    assert.equal(fn(0, 1), 0);
    assert.throws(() => fn(-1, 5), RangeError);
    assert.throws(() => fn(5, 0), RangeError);
    assert.throws(() => fn(5, -1), RangeError);
  });
}

