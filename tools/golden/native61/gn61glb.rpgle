**FREE
// Stage 2, S2.61: procedures that read and change the module's global
// fields (a counter, a data structure subfield, an array element, a
// character field), with a local of the same name as a global hiding it.
// Captured on PUB400 (tools/golden/native61.txt n61_glb) for the native
// unit NGLOBAL.
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
say('CALLS ' + %char(calls) + ' TOTAL ' + %char(total));
say('LABEL [' + %trim(label) + '] COUNT ' + %char(stats.count) + ' LAST ' + stats.last);
say('HITS ' + %char(hits(1)) + ' ' + %char(hits(2)) + ' ' + %char(hits(3)));
say('SHADOW ' + %char(shadow) + ' LOCAL ' + %char(local7()));
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

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
