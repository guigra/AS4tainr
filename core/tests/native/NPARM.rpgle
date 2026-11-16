**FREE
// Stage 2, S2.51: pointer parameters, lowered to Rust. (The capture's
// QCMDEXC with OPTIONS(*VARSIZE) needs a job, which this harness does not
// give; the suite's own *VARSIZE tests run in jobs, native-diff level 2.)
// Expected: what IBM i 7.5 gave (golden capture tools/golden/native52.txt
// n52_parm, cited in native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s buf char(6) inz('ABCDEF');
dcl-s buf2 char(6) inz('XYZXYZ');
dcl-s p pointer;
p = %addr(buf);
dsply ('FIRST [' + first3(p) + ']');
dsply ('ADDR [' + first3(%addr(buf2)) + ']');
dsply ('MOVED [' + first3(%addr(buf) + 2) + ']');
point(p);
dsply ('POINTED [' + first3(p) + ']');
dsply ('NULL ' + isnull(*null));
dsply ('SET ' + isnull(p));
*inlr = *on;
dcl-proc first3;
  dcl-pi *n char(3);
    at pointer value;
  end-pi;
  dcl-s c char(3) based(at);
  return c;
end-proc;
dcl-proc point;
  dcl-pi *n;
    q pointer;
  end-pi;
  q = %addr(buf2);
end-proc;
dcl-proc isnull;
  dcl-pi *n char(3);
    at pointer value;
  end-pi;
  if at = *null;
    return 'YES';
  endif;
  return 'NO';
end-proc;
