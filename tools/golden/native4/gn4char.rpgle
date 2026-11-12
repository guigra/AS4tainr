**FREE
// Character fields: EVAL and EVALR (padding, truncation), concatenation
// of fixed-length fields (their trailing blanks kept), and comparisons of
// different lengths.
ctl-opt dftactgrp(*no);
dcl-s a char(5) inz('AB');
dcl-s b char(3) inz('XYZ');
dcl-s c char(10);
dcl-s d char(4);
dcl-s e char(7) inz('AB');
c = a + b;
say('CHR CAT [' + c + ']');
d = a + b;
say('CHR TRUNC [' + d + ']');
evalr c = b;
say('CHR EVALR [' + c + ']');
evalr d = a + b;
say('CHR EVALR TRUNC [' + d + ']');
c = 'Q';
say('CHR PAD [' + c + ']');
if a = e;
  say('CHR CMP PADDED EQ');
else;
  say('CHR CMP PADDED NE');
endif;
if a < b;
  say('CHR CMP LT');
else;
  say('CHR CMP GE');
endif;
if 'ab' < 'AB';
  say('CHR CMP LOWER FIRST');
else;
  say('CHR CMP UPPER FIRST');
endif;
if '1' > 'Z';
  say('CHR CMP DIGIT HIGH');
else;
  say('CHR CMP DIGIT LOW');
endif;
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
