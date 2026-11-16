**FREE
// Stage 2, S2.47: pointers. %ADDR and a BASED field over it, a pointer
// copied, %ADDR(varying:*DATA), *NULL and comparisons with it. Captured
// on PUB400 (tools/golden/native48.txt n48_ptr) for the native unit NPTR.
ctl-opt dftactgrp(*no);
dcl-s x char(5) inz('hello');
dcl-s y varchar(10) inz('abc');
dcl-s p pointer;
dcl-s q pointer;
dcl-s bx char(5) based(p);
dcl-s bq char(3) based(q);
dcl-s t varchar(40);
if p = *null;
  say('START NULL');
else;
  say('START SET');
endif;
p = %addr(x);
say('P [' + bx + ']');
q = p;
say('Q [' + bq + ']');
q = %addr(y:*data);
say('DATA [' + bq + ']');
if p = *null;
  t = 'P NULL';
else;
  t = 'P SET';
endif;
if p = q;
  t += ' SAME';
else;
  t += ' DIFFERENT';
endif;
say(t);
p = *null;
if p = *null;
  say('AFTER NULL');
else;
  say('AFTER SET');
endif;
x = 'world';
p = %addr(x);
say('AGAIN [' + bx + ']');
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
