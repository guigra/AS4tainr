**FREE
// Stage 2, S2.9: %TRIM, %TRIML, %TRIMR and %LEN of character fields.
// Captured on PUB400 (tools/golden/native6.txt n6_bif) for the native
// unit test NBIF.
ctl-opt dftactgrp(*no);
dcl-s a char(6) inz('  AB  ');
dcl-s b char(3) inz('XY');
dcl-s c char(10);
dcl-s z char(4);
dcl-s n packed(5:0);
c = %trim(a);
say('TRIM [' + c + ']');
c = %triml(a);
say('TRIML [' + c + ']');
c = %trimr(a);
say('TRIMR [' + c + ']');
c = %trimr(b) + %trim(a);
say('TRIM CAT [' + c + ']');
c = %trim(z) + 'Q';
say('TRIM BLANKS [' + c + ']');
evalr c = %trim(a);
say('TRIM EVALR [' + c + ']');
n = %len(a);
say('LEN ' + %char(n));
n = %len(a) + %len(b) * 2;
say('LEN EXPR ' + %char(n));
*inlr = *on;
// Write one line to gn1.out in the job's current directory (DSPLY under
// `system` goes to QSYSOPR, a queue PUB400 users share).
dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
