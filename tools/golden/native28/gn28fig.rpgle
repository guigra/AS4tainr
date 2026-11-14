**FREE
// Stage 2, S2.22: figurative constants in EVAL and comparisons. Captured
// on PUB400 (tools/golden/native28.txt n28_fig) for the native unit test
// NFIG.
ctl-opt dftactgrp(*no);
dcl-s c char(5) inz('ABCDE');
dcl-s p packed(5:2) inz(123.45);
dcl-s z zoned(3:0) inz(7);
dcl-s v varchar(5) inz('XY');
c = *blanks;
say('FIG BLANKS [' + c + ']');
if c = *blanks;
  say('FIG CMP BLANKS EQ');
else;
  say('FIG CMP BLANKS NE');
endif;
p = *zeros;
say('FIG ZEROS ' + %char(p));
z = *hival;
say('FIG HIVAL ' + %char(z));
if z = *hival;
  say('FIG CMP HIVAL EQ');
else;
  say('FIG CMP HIVAL NE');
endif;
p = *loval;
say('FIG LOVAL ' + %char(p));
c = *all'AB';
say('FIG ALL [' + c + ']');
if c > *loval and c < *hival;
  say('FIG CMP RANGE YES');
else;
  say('FIG CMP RANGE NO');
endif;
v = *blanks;
say('FIG VAR [' + v + '] ' + %char(%len(v)));
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
