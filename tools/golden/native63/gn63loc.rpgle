**FREE
// Stage 2, S2.63: arrays of a procedure's own storage. A local array with
// literal and field indexes, a local data structure array's subfields,
// a recursive call whose own array is kept apart from its caller's, and
// an index out of range under MONITOR (status 00121). Captured on PUB400
// (tools/golden/native63.txt n63_loc) for the native unit NLOCARR.
ctl-opt dftactgrp(*no);
say('SUM ' + %char(fill(4)));
say('DEPTH ' + %char(depth(3)));
say('DS ' + pairs());
say('RANGE ' + %char(range(7)));
*inlr = *on;

dcl-proc fill;
  dcl-pi *n int(10);
    n int(10) value;
  end-pi;
  dcl-s a packed(5 : 0) dim(5);
  dcl-s i int(10);
  dcl-s t int(10);
  for i = 1 to n;
    a(i) = i * 10;
  endfor;
  a(5) = 7;
  for i = 1 to 5;
    t += a(i);
  endfor;
  return t;
end-proc;

dcl-proc depth;
  dcl-pi *n int(10);
    n int(10) value;
  end-pi;
  dcl-s mine int(10) dim(2);
  mine(1) = n;
  mine(2) = n * 100;
  if n > 1;
    mine(2) += depth(n - 1);
  endif;
  return mine(1) + mine(2);
end-proc;

dcl-proc pairs;
  dcl-pi *n varchar(40) end-pi;
  dcl-ds p qualified dim(3);
    k char(2);
    v packed(3 : 0);
  end-ds;
  dcl-s i int(10);
  dcl-s res varchar(40);
  for i = 1 to 3;
    p(i).k = 'K' + %char(i);
    p(i).v = i * 11;
  endfor;
  p(2).v = 99;
  for i = 1 to 3;
    res += p(i).k + '=' + %char(p(i).v) + ' ';
  endfor;
  return res;
end-proc;

dcl-proc range;
  dcl-pi *n int(10);
    k int(10) value;
  end-pi;
  dcl-s a int(10) dim(3);
  monitor;
    a(k) = 1;
  on-error;
    return %status;
  endmon;
  return 0;
end-proc;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
