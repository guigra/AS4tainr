**FREE
// Stage 2, S2.97: local procedures taking a LIKEDS data structure by
// reference: BUMP reads and changes its subfields, and the caller sees
// the changes; it is passed a global data structure, a qualified one,
// and from another procedure that procedure's own local one. Captured
// on PUB400 (tools/golden/native98.txt n98_dsparm) for the native unit
// NDSPARM.
ctl-opt dftactgrp(*no);
dcl-ds t qualified template;
  a char(3);
  n packed(5:0);
end-ds;
dcl-ds g likeds(t) inz;
dcl-ds h qualified;
  a char(3) inz('hhh');
  n packed(5:0) inz(10);
end-ds;
dcl-s line varchar(80);
g.a = 'ggg';
g.n = 1;
bump(g);
bump(g);
bump(h);
line = g.a + %char(g.n) + ' ' + h.a + %char(h.n) + ' ' + inner();
say(line);
*inlr = *on;

dcl-proc bump;
  dcl-pi *n;
    d likeds(t);
  end-pi;
  d.n += 5;
  d.a = %subst(d.a: 2: 2) + %char(d.n / 5);
end-proc;

dcl-proc inner;
  dcl-pi *n varchar(20);
  end-pi;
  dcl-ds loc likeds(t);
  loc.a = 'LLL';
  loc.n = 100;
  bump(loc);
  return loc.a + %char(loc.n);
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
