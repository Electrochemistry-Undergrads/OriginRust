// Our own Origin C helper for the power and sample size reference runs (stats4-pss*.lines.txt).
// Column 1 of the active sheet holds the sample sizes (mode 0) or the powers (mode 1); the library result goes
// to column 2 and the lowest power (mode 1) to column 3. Only documented library functions are called.
#include <Origin.h>
#include <stats_utils.h>

static void put(Worksheet &wks, int col, const vector<double> &v)
{
	while (wks.GetNumCols() <= col)
		wks.AddCol();
	Dataset ds(wks, col);
	ds.SetSize(v.GetSize());
	for (int ip = 0; ip < v.GetSize(); ip++)
		ds[ip] = v[ip];
}

// kind: 1 one-sample t, 2 two-sample t, 3 paired t, 4 one-way ANOVA; mode 0 power, 1 sample size;
// tail 0 two-sided, 1 upper; (a, b, sd): Mean1, Mean2, SD for the t tests, Level, CSS, SD for the ANOVA.
double sheaf_pss(int kind, int mode, int tail, double alpha, double a, double b, double sd, double hv)
{
	Worksheet wks = Project.ActiveLayer();
	Dataset ds(wks, 0);
	vector<double> in, out, low;
	for (int i = 0; i < ds.GetSize(); i++)
		if (!is_missing_value(ds[i]))
			in.Add(ds[i]);
	int n = in.GetSize();
	out.SetSize(n);
	low.SetSize(1);
	low[0] = -1;
	PowerOpts po;
	po.Mean1 = a;
	po.Mean2 = b;
	po.SD = sd;
	ANOVAPowerOpts ao;
	ao.Level = a;
	ao.CSS = b;
	ao.SD = sd;
	HypotTestOptions ho;
	ho.HypotValue = hv;
	ho.TailType = tail;
	ho.TestType = 0;
	int rc = -999;
	double lowest = -1;
	if (kind == 1)
	{
		if (mode == 0)
			rc = ocmath_tTest_powers1(alpha, tail, in, n, &po, out);
		else
			rc = ocmath_tTest_samplesize(alpha, tail, in, n, &po, out, &lowest);
	}
	else if (kind == 2)
	{
		if (mode == 0)
			rc = ocmath_two_sample_tTest_powers1(alpha, &ho, in, n, &po, out);
		else
			rc = ocmath_two_sample_tTest_samplesize(alpha, &ho, in, n, &po, out, &lowest);
	}
	else if (kind == 3)
	{
		if (mode == 0)
			rc = ocmath_two_sample_tTest_powers_pair1(alpha, &ho, in, n, &po, out);
		else
			rc = ocmath_two_sample_tTest_samplesize_pair(alpha, &ho, in, n, &po, out, &lowest);
	}
	else
	{
		if (mode == 0)
			rc = ocmath_anova_powers1(alpha, in, n, &ao, out);
		else
			rc = ocmath_anova_samplesize(alpha, in, n, &ao, out);
	}
	low[0] = lowest;
	vector<double> r;
	r.Add(rc);
	put(wks, 1, out);
	put(wks, 2, low);
	put(wks, 3, r);
	return rc;
}
