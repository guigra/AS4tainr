**FREE
// Interpreter fix (fork branch fix/compile-checks): a date plus a time,
// which compiles, and runs: the timestamp. Captured on PUB400
// (tools/golden/native217.txt).
ctl-opt dftactgrp(*no);
dcl-s d date inz(d'2026-01-31');
dcl-s t time inz(t'09.15.00');
dcl-s z timestamp;

z = d + t;
dsply (%char(z));
*inlr = *on;
