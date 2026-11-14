**FREE
// Stage 2, S2.24: a whole data structure as a character operand: assigned
// (padded, EVALR), read, compared, with %SUBST and %LEN. Captured on
// PUB400 (tools/golden/native31.txt n31_dsw) for the native unit test NDSW.
ctl-opt dftactgrp(*no);
dcl-ds rec;
  code char(3);
  qty zoned(3:0);
  name char(4);
end-ds;
dcl-s c char(12);
rec = 'ABC012WXYZ';
say('DSW SUBS [' + code + '|' + %char(qty) + '|' + name + ']');
c = rec;
say('DSW TO CHAR [' + c + ']');
rec = 'XY';
say('DSW PAD [' + %subst(rec:1:3) + ']');
if rec = 'XY';
  say('DSW CMP PADDED EQ');
else;
  say('DSW CMP PADDED NE');
endif;
qty = 7;
c = rec;
say('DSW AFTER SUB [' + c + ']');
evalr rec = 'Q';
say('DSW EVALR [' + name + ']');
say('DSW LEN ' + %char(%len(rec)));
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
