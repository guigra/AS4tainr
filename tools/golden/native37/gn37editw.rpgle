**FREE
// Stage 2, S2.29: %EDITW with thousands separators, CR, minus, asterisk
// fill, a floating currency symbol, ampersands, zero, an integer and an
// expression. Captured on PUB400 (tools/golden/native37.txt n37_editw) for
// the native unit test NEDITW.
ctl-opt dftactgrp(*no);
dcl-s p packed(9:2) inz(-1234.56);
dcl-s q packed(9:2) inz(1234.56);
dcl-s z packed(9:2) inz(0);
dcl-s i int(10) inz(1234567);
dcl-s r varchar(40);
r = %editw(p:'   ,   ,  0.  ');
say('EW PLAIN [' + r + ']');
r = %editw(p:'   ,   ,  0.  CR');
say('EW CR [' + r + ']');
r = %editw(p:'   ,   ,  0.  -');
say('EW MINUS [' + r + ']');
r = %editw(p:'   ,   ,  *.  ');
say('EW ASTER [' + r + ']');
r = %editw(p:'   ,   , $0.  ');
say('EW DOLLAR [' + r + ']');
r = %editw(p:'   &   &  0.  ');
say('EW AMP [' + r + ']');
r = %editw(z:'   ,   ,  0.  ');
say('EW ZERO [' + r + ']');
r = %editw(z:'   ,   ,  0.  CR');
say('EW ZERO CR [' + r + ']');
r = %editw(i:'    ,   ,   ');
say('EW INT [' + r + ']');
r = %editw(q:'   ,   ,  0.  CR');
say('EW POS CR [' + r + ']');
r = %editw(p * 2:'    ,   ,  0.  ');
say('EW EXPR [' + r + ']');
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
