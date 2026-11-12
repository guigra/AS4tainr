**FREE
// Stage 2, S2.14: %CHAR under DECEDIT('0,'). Captured on PUB400
// (tools/golden/native12.txt n12_decedit) for the native unit test NFUN2.
ctl-opt dftactgrp(*no) decedit('0,');
dcl-s z zoned(5:2) inz(0.25);
dcl-s d52 packed(5:2) inz(-12.50);
dcl-s c char(12);
c = %char(z);
say('DECEDIT FRACTION [' + c + ']');
c = %char(d52);
say('DECEDIT NEG [' + c + ']');
*inlr = *on;
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
