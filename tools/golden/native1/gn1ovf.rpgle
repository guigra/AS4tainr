**FREE
// Unmonitored overflow of a packed(3:0) field: the escape message.
dcl-s small packed(3:0) inz(999);
small = small + 1;
*inlr = *on;
