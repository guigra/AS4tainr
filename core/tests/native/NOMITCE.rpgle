**FREE
// Native unit NOMITCE (S2.150): *OMIT arguments to CEE APIs, the program
// captured on PUB400 as GN150OM with DSPLY for its output.
// Source: golden capture tools/golden/native150 n150_omit (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-pr ceedays extproc('CEEDAYS') opdesc;
  indate char(255) const options(*varsize);
  picture char(255) const options(*varsize);
  lil int(10);
  fc char(12) options(*omit);
end-pr;
dcl-pr ceedywk extproc('CEEDYWK');
  lil int(10) const;
  wday int(10);
  fc char(12) options(*omit);
end-pr;
dcl-pr ceedate extproc('CEEDATE') opdesc;
  lil int(10) const;
  picture char(255) const options(*varsize);
  outdt char(80) options(*varsize);
  fc char(12) options(*omit);
end-pr;
dcl-s lil int(10);
dcl-s wday int(10);
dcl-s outdt char(80);
dcl-s fc char(12);
dcl-s line varchar(80);

ceedays('2026-01-31': 'YYYY-MM-DD': lil: *omit);
ceedywk(lil: wday: *omit);
ceedate(lil + 1: 'YYYY-MM-DD': outdt: fc);
line = %char(lil) + ' ' + %char(wday) + ' ' + %trimr(outdt);
dsply (line);
*inlr = *on;
