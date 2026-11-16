**FREE
// Stage 2, S2.48: BASED fields, lowered to Rust. Expected: what IBM i 7.5
// gave (golden capture tools/golden/native49.txt n49_bas, cited in
// native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s buf char(10) inz('ABCDEFGHIJ');
dcl-s pk char(3) inz(x'12345F');
dcl-s p pointer;
dcl-s q pointer;
dcl-s part char(4) based(p);
dcl-ds over based(q) qualified;
  first char(2);
  amount packed(5:0);
end-ds;
dcl-s raw char(5);
dcl-s n packed(7:0);
p = %addr(buf);
dsply ('PART [' + part + ']');
part = 'wxyz';
dsply ('BUF [' + buf + ']');
p = %addr(buf) + 6;
dsply ('SHIFT [' + part + ']');
raw = 'XY' + pk;
q = %addr(raw);
n = over.amount + 1;
dsply ('FIRST [' + over.first + '] AMOUNT ' + %char(n));
over.amount = 999;
dsply ('RAW HEX ' + %char(%len(raw)) + ' [' + over.first + '] ' + %char(over.amount));
dsply ('LOCAL [' + upper4(%addr(buf)) + ']');
dsply ('AFTER [' + buf + ']');
p = *null;
monitor;
  dsply ('NULL [' + part + ']');
on-error;
  dsply ('NULL ERROR ' + %char(%status));
endmon;
*inlr = *on;
dcl-proc upper4;
  dcl-pi *n char(4);
    at pointer value;
  end-pi;
  dcl-s here char(4) based(at);
  here = 'QRST';
  return here;
end-proc;
