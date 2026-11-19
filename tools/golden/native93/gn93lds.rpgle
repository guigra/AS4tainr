**FREE
// Stage 2, S2.93: a procedure's own data structures as a whole. In
// TAKE, a local qualified data structure is given %SUBST of a buffer and
// its subfields read; it is read back whole into a character field. In
// FILL, a LIKEDS parameter by reference is given a value whole, and the
// caller sees it. Captured on PUB400 (tools/golden/native93.txt
// n93_localds) for the native unit NLOCDS.
ctl-opt dftactgrp(*no);
dcl-ds t qualified template;
  a char(3);
  n zoned(3:0);
end-ds;
dcl-ds g likeds(t);
dcl-s line varchar(80);
line = take('ABC012XYZ');
fill(g);
line += ' ' + g.a + '/' + %char(g.n);
say(line);
*inlr = *on;

dcl-proc take;
  dcl-pi *n varchar(40);
    buf char(20) const;
  end-pi;
  dcl-ds h qualified;
    a char(3);
    n zoned(3:0);
  end-ds;
  dcl-s c char(6);
  h = %subst(buf: 1: %size(h));
  c = h;
  return h.a + '|' + %char(h.n + 1) + '|' + c;
end-proc;

dcl-proc fill;
  dcl-pi *n;
    d likeds(t);
  end-pi;
  d = 'QRS042';
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
