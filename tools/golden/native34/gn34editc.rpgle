**FREE
// Stage 2, S2.26: %EDITC with each edit code, of a negative value and of
// zero, a date (Y), a literal, an expression and an integer. Captured on
// PUB400 (tools/golden/native34.txt n34_editc) for the native unit test
// NEDITC.
ctl-opt dftactgrp(*no);
dcl-s p packed(9:2) inz(-1234.56);
dcl-s z packed(9:2) inz(0);
dcl-s dt zoned(6:0) inz(31226);
dcl-s i int(10) inz(-1234567);
dcl-s r varchar(40);
r = %editc(p:'1');
say('EDITC 1 NEG [' + r + ']');
r = %editc(p:'2');
say('EDITC 2 NEG [' + r + ']');
r = %editc(p:'3');
say('EDITC 3 NEG [' + r + ']');
r = %editc(p:'4');
say('EDITC 4 NEG [' + r + ']');
r = %editc(p:'A');
say('EDITC A NEG [' + r + ']');
r = %editc(p:'B');
say('EDITC B NEG [' + r + ']');
r = %editc(p:'C');
say('EDITC C NEG [' + r + ']');
r = %editc(p:'D');
say('EDITC D NEG [' + r + ']');
r = %editc(p:'J');
say('EDITC J NEG [' + r + ']');
r = %editc(p:'K');
say('EDITC K NEG [' + r + ']');
r = %editc(p:'L');
say('EDITC L NEG [' + r + ']');
r = %editc(p:'M');
say('EDITC M NEG [' + r + ']');
r = %editc(p:'N');
say('EDITC N NEG [' + r + ']');
r = %editc(p:'O');
say('EDITC O NEG [' + r + ']');
r = %editc(p:'P');
say('EDITC P NEG [' + r + ']');
r = %editc(p:'Q');
say('EDITC Q NEG [' + r + ']');
r = %editc(p:'X');
say('EDITC X NEG [' + r + ']');
r = %editc(p:'Z');
say('EDITC Z NEG [' + r + ']');
r = %editc(z:'1');
say('EDITC 1 ZERO [' + r + ']');
r = %editc(z:'2');
say('EDITC 2 ZERO [' + r + ']');
r = %editc(z:'3');
say('EDITC 3 ZERO [' + r + ']');
r = %editc(z:'4');
say('EDITC 4 ZERO [' + r + ']');
r = %editc(z:'A');
say('EDITC A ZERO [' + r + ']');
r = %editc(z:'B');
say('EDITC B ZERO [' + r + ']');
r = %editc(z:'J');
say('EDITC J ZERO [' + r + ']');
r = %editc(z:'K');
say('EDITC K ZERO [' + r + ']');
r = %editc(z:'N');
say('EDITC N ZERO [' + r + ']');
r = %editc(z:'O');
say('EDITC O ZERO [' + r + ']');
r = %editc(z:'Z');
say('EDITC Z ZERO [' + r + ']');
r = %editc(dt:'Y');
say('EDITC Y DATE [' + r + ']');
r = %editc(12345:'3');
say('EDITC LIT [' + r + ']');
r = %editc(p * 2:'J');
say('EDITC EXPR [' + r + ']');
r = %editc(i:'1');
say('EDITC INT [' + r + ']');
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
