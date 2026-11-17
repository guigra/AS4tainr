**FREE
// Native unit NGLOBAL (S2.61): procedures that read and change the
// module's global fields (a counter, a data structure subfield, an array
// element, a character field), with a local of the same name as a global
// hiding it. The program captured on PUB400, with DSPLY for its output.
// Source: golden capture tools/golden/native61 n61_glb (PUB400, IBM i
// 7.5, 2026-11-17).
ctl-opt dftactgrp(*no);
dcl-s calls int(10) inz(0);
dcl-s total packed(9 : 2) inz(0);
dcl-s label char(12) inz('start');
dcl-s hits int(10) dim(3);
dcl-s shadow int(10) inz(7);
dcl-ds stats qualified;
  count int(10);
  last char(5);
end-ds;
add(10.25);
add(2.50);
add(-1);
dsply ('CALLS ' + %char(calls) + ' TOTAL ' + %char(total));
dsply ('LABEL [' + %trim(label) + '] COUNT ' + %char(stats.count) + ' LAST ' + stats.last);
dsply ('HITS ' + %char(hits(1)) + ' ' + %char(hits(2)) + ' ' + %char(hits(3)));
dsply ('SHADOW ' + %char(shadow) + ' LOCAL ' + %char(local7()));
*inlr = *on;

dcl-proc add;
  dcl-pi *n;
    amt packed(7 : 2) const;
  end-pi;
  calls += 1;
  total += amt;
  stats.count = calls;
  if amt < 0;
    stats.last = 'NEG';
    hits(3) += 1;
  else;
    stats.last = 'POS';
    hits(1) += 1;
  endif;
  label = 'after ' + %char(calls);
end-proc;

dcl-proc local7;
  dcl-pi *n int(10) end-pi;
  dcl-s shadow int(10);
  shadow = 70;
  return shadow + 1;
end-proc;
