**FREE
// Stage 2, S2.27: %LOOKUP and %LOOKUPLT/LE/GE/GT in character and
// ascending numeric arrays, with a start and a number of elements.
// Captured on PUB400 (tools/golden/native35.txt n35_look) for the native
// unit test NLOOK.
ctl-opt dftactgrp(*no);
dcl-s arr char(3) dim(5);
dcl-s nums packed(5:0) dim(5) ascend;
dcl-s x char(3) inz('EEE');
dcl-s s3 int(10) inz(3);
dcl-s n int(10);
arr(1) = 'AAA';
arr(2) = 'BBB';
arr(3) = 'CCC';
arr(4) = 'BBB';
arr(5) = 'EEE';
nums(1) = 10;
nums(2) = 20;
nums(3) = 30;
nums(4) = 40;
nums(5) = 50;
n = %lookup('BBB':arr);
say('LOOK EQ ' + %char(n));
n = %lookup('BBB':arr:3);
say('LOOK START ' + %char(n));
n = %lookup('ZZZ':arr);
say('LOOK NONE ' + %char(n));
n = %lookup('BBB':arr:1:1);
say('LOOK RANGE ' + %char(n));
n = %lookup(x:arr);
say('LOOK VAR ' + %char(n));
n = %lookuplt(25:nums);
say('LOOK LT ' + %char(n));
n = %lookuple(30:nums);
say('LOOK LE ' + %char(n));
n = %lookupge(35:nums);
say('LOOK GE ' + %char(n));
n = %lookupgt(50:nums);
say('LOOK GT NONE ' + %char(n));
n = %lookupgt(5:nums);
say('LOOK GT FIRST ' + %char(n));
n = %lookup(40:nums);
say('LOOK NUM EQ ' + %char(n));
n = %lookup('BBB':arr:s3);
say('LOOK START VAR ' + %char(n));
if %lookup(x:arr) > 0;
  say('LOOK COND YES');
else;
  say('LOOK COND NO');
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
