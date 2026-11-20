**FREE
// Stage 2, S2.146: FOR with arithmetic in its limit: %LEN minus one, a
// product down to a difference, and a difference of a product.
// Captured on PUB400 (tools/golden/native146.txt n146_forlim) for the
// native unit NFORLIM.
ctl-opt dftactgrp(*no);
dcl-s s varchar(10) inz('abcde');
dcl-s i int(10);
dcl-s n packed(3:0) inz(4);
dcl-s line varchar(80);

for i = 1 to %len(s) - 1;
  line += %subst(s: i: 1);
endfor;
for i = n * 2 downto n - 1;
  line += %char(i);
endfor;
for i = 0 to 10 - n * 2;
  line += %char(i);
endfor;
say(line);
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
