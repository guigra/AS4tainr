**FREE
// Native unit NFLTDSP (S2.183): DSPLY of float fields, the values of the
// program captured on PUB400 as GN183FD (there DSPLY sent them to a
// message queue, here to the job's DSPLY output).
// Source: golden capture tools/golden/native183 n183_fltdsply (PUB400,
// IBM i 7.5, 2026-11-21).
dcl-s f8 float(8) inz(123);
dcl-s f4 float(4) inz(0.5);
dcl-s g8 float(8) inz(-0.000125);

dsply f8;
dsply f4;
dsply g8;
*inlr = *on;
