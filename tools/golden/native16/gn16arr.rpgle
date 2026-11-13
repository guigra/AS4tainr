**FREE
// Stage 2, S2.15: array elements with literal indexes, of numeric and
// character arrays and of an array subfield. Captured on PUB400
// (tools/golden/native16.txt n16_arr) for the native unit test NARR.
ctl-opt dftactgrp(*no);
dcl-s arr packed(5:2) dim(5);
dcl-s names char(4) dim(3);
dcl-s tot packed(7:2);
dcl-s i int(10);
dcl-ds d;
  sub char(2) dim(3);
end-ds;
arr(1) = 1.25;
arr(5) = arr(1) * 2;
say('ARR FIRST LAST ' + %char(arr(1)) + ' ' + %char(arr(5)));
names(2) = 'AB';
names(3) = %trim(names(2)) + 'C';
say('ARR CHAR [' + names(1) + '|' + names(2) + '|' + names(3) + ']');
if arr(5) > arr(1);
  say('ARR COND GT');
else;
  say('ARR COND LE');
endif;
arr(3) = arr(5) - arr(1);
for i = 1 to 5;
  tot = tot + arr(i);
endfor;
say('ARR SUM ' + %char(tot));
sub(2) = 'XY';
sub(1) = sub(2);
say('ARR DS [' + d + ']');
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
