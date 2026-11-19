**FREE
// Stage 2, S2.133: calls of a procedure returning an indicator as
// conditions (IF, NOT, AND, DOW) and in %CHAR. Captured on PUB400
// (tools/golden/native133.txt n133_indcall) for the native unit NINDCALL.
ctl-opt dftactgrp(*no);
dcl-s n int(10);
dcl-s line varchar(80);

if isbig(7);
  line += 'a';
endif;
if not isbig(2);
  line += 'b';
endif;
if isbig(9) and isbig(1);
  line += 'c';
endif;
dow not isbig(n);
  n += 2;
enddo;
line += %char(n) + %char(isbig(n));
say(line);
*inlr = *on;

dcl-proc isbig;
  dcl-pi *n ind;
    x int(10) value;
  end-pi;
  return x > 5;
end-proc;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
