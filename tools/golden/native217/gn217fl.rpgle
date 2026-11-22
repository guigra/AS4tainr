**FREE
// Interpreter fix (fork branch fix/compile-checks): a FOR limit that is
// not whole. Expected not to compile (RNF7569). Captured on PUB400
// (tools/golden/native217.txt).
ctl-opt dftactgrp(*no);
dcl-s i int(10);
dcl-s c int(10);

for i = 0 to 10 / 4;
  c += 1;
endfor;
*inlr = *on;
