**FREE
// Stage 2, S2.208: a whole array for a CONST array parameter of its
// elements' type: an array of 2 and one of 5 elements given to
// int(10) CONST DIM(5) OPTIONS(*VARSIZE), and a procedure's own array.
// Captured on PUB400 (tools/golden/native208.txt n208_wholearr) for the
// native unit NWHOLEA.
ctl-opt dftactgrp(*no);
dcl-pr sumk int(10);
  n int(10) const;
  keys int(10) const dim(5) options(*varsize);
end-pr;
dcl-s two int(10) dim(2);
dcl-s five int(10) dim(5);
dcl-s line varchar(200);

two(1) = 3;
two(2) = 4;
five(1) = 1;
five(2) = 2;
five(3) = 3;
five(4) = 4;
five(5) = 5;
line = %char(sumk(2 : two)) + ' ' + %char(sumk(5 : five)) + ' ' + %char(local());
say(line);
*inlr = *on;

dcl-proc sumk;
  dcl-pi *n int(10);
    n int(10) const;
    keys int(10) const dim(5) options(*varsize);
  end-pi;
  dcl-s i int(10);
  dcl-s t int(10);
  for i = 1 to n;
    t += keys(i);
  endfor;
  return t;
end-proc;

dcl-proc local;
  dcl-pi *n int(10) end-pi;
  dcl-s k int(10) dim(1);
  k(1) = 40;
  return sumk(1 : k);
end-proc;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
