**FREE
// Stage 2, S2.21: array elements with an index that is a field: in FOR
// loops, as targets, in conditions, and out of range. Captured on PUB400
// (tools/golden/native27.txt n27_arrv) for the native unit test NARRV.
ctl-opt dftactgrp(*no);
dcl-s arr packed(5:0) dim(5);
dcl-s names char(3) dim(4);
dcl-s i int(10);
dcl-s j int(10);
dcl-s tot packed(7:0);
for i = 1 to 5;
  arr(i) = i * 10;
endfor;
for i = 1 to 5;
  tot = tot + arr(i);
endfor;
say('ARRV SUM ' + %char(tot));
i = 3;
names(i) = 'ABC';
j = 2;
names(j) = names(i);
say('ARRV CHAR [' + names(1) + '|' + names(2) + '|' + names(3) + ']');
if arr(i) > 20;
  say('ARRV COND GT');
else;
  say('ARRV COND LE');
endif;
arr(j) = arr(i) + arr(j);
say('ARRV EXPR ' + %char(arr(2)));
i = 6;
monitor;
  tot = arr(i);
  say('ARRV PAST ' + %char(tot));
on-error;
  say('ARRV PAST ERR ' + %char(%status));
endmon;
i = 0;
monitor;
  arr(i) = 1;
  say('ARRV ZERO OK');
on-error;
  say('ARRV ZERO ERR ' + %char(%status));
endmon;
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
