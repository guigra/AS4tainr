**FREE
// Stage 2, S2.81: %EDITC with a third argument: *ASTFILL, *CURSYM and a
// one-character literal, with edit codes J, 1, A, 3 and K (not Z: RNF6068) over a
// positive, a negative and a zero value of a packed(7:2) field. Each
// result shows between bars. Captured on PUB400 (tools/golden/native81.txt
// n81_editc) for the native unit NEDITC3.
ctl-opt dftactgrp(*no);
dcl-s v packed(7:2) dim(3);
dcl-s i int(10);
dcl-s x packed(7:2);
v(1) = 1234.5;
v(2) = -42;
v(3) = 0;
for i = 1 to 3;
  x = v(i);
  say('A|' + %editc(x: 'J': *astfill) + '|' + %editc(x: '1': *astfill) + '|');
  say('C|' + %editc(x: 'J': *cursym) + '|' + %editc(x: 'A': *cursym) + '|');
  say('L|' + %editc(x: '3': '+') + '|' + %editc(x: 'K': '+') + '|');
endfor;
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
